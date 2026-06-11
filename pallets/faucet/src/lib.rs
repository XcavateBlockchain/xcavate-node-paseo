// Xcavate Protocol - https://xcavate.io/
// Copyright (C) 2025, Xcavate Foundation

// The Xcavate Protocol is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Xcavate Protocol is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

use frame_support::{
    pallet_prelude::*,
    sp_runtime::traits::BlockNumberProvider,
    traits::{
        fungible::{self, Inspect as FungibleInspect},
        fungibles::Mutate as FungiblesMutate,
        tokens::{fungibles, Balance},
    },
};

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Type representing the weight of this pallet.
        type WeightInfo: WeightInfo;

        /// The type used to represent balances.
        type Balance: Balance
            + TypeInfo
            + From<u128>
            + Into<<Self as pallet::Config>::Balance>
            + Default;

        /// The native currency for balance checks.
        type NativeCurrency: fungible::Inspect<AccountIdOf<Self>, Balance = <Self as pallet::Config>::Balance>
            + fungible::Mutate<AccountIdOf<Self>>;

        /// The foreign assets pallet for minting tGBP.
        type ForeignCurrency: fungibles::Inspect<
                AccountIdOf<Self>,
                AssetId = u32,
                Balance = <Self as pallet::Config>::Balance,
            > + FungiblesMutate<
                AccountIdOf<Self>,
                AssetId = u32,
                Balance = <Self as pallet::Config>::Balance,
            > + fungibles::Create<AccountIdOf<Self>>;

        /// The asset ID to drip.
        #[pallet::constant]
        type DripAssetId: Get<u32>;

        /// Amount to drip per claim.
        #[pallet::constant]
        type DripAmount: Get<<Self as pallet::Config>::Balance>;

        /// Minimum XCAV balance required to claim.
        #[pallet::constant]
        type MinXcavBalance: Get<<Self as pallet::Config>::Balance>;

        /// Cooldown period in blocks between claims.
        #[pallet::constant]
        type CooldownPeriod: Get<BlockNumberFor<Self>>;

        /// Provider for the current block number.
        type BlockNumberProvider: BlockNumberProvider<BlockNumber = BlockNumberFor<Self>>;
    }

    /// Last block at which an account claimed from the faucet.
    #[pallet::storage]
    pub type LastClaim<T: Config> =
        StorageMap<_, Blake2_128Concat, AccountIdOf<T>, BlockNumberFor<T>, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Tokens dripped to an account.
        Dripped { who: T::AccountId, amount: T::Balance },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Caller does not hold enough XCAV.
        InsufficientXcavBalance,
        /// Cooldown period has not elapsed since the last claim.
        CooldownNotElapsed,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Claim tGBP from the faucet.
        ///
        /// The caller must hold at least `MinXcavBalance` of XCAV and the
        /// cooldown period must have elapsed since their last claim.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::drip())]
        pub fn drip(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let xcav_balance = T::NativeCurrency::balance(&who);
            ensure!(xcav_balance >= T::MinXcavBalance::get(), Error::<T>::InsufficientXcavBalance);

            let current_block = T::BlockNumberProvider::current_block_number();
            if let Some(last) = LastClaim::<T>::get(&who) {
                ensure!(
                    current_block >= last + T::CooldownPeriod::get(),
                    Error::<T>::CooldownNotElapsed
                );
            }

            let amount = T::DripAmount::get();
            T::ForeignCurrency::mint_into(T::DripAssetId::get(), &who, amount)?;

            LastClaim::<T>::insert(&who, current_block);

            Self::deposit_event(Event::<T>::Dripped { who, amount });
            Ok(())
        }
    }
}
