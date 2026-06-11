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

//! Benchmarking setup for pallet-faucet
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as Faucet;
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_support::traits::{fungible::Mutate, fungibles::Create};
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn drip() {
        let caller: T::AccountId = account("caller", 0, 0);
        let admin: T::AccountId = account("admin", 0, 1);

        let min_balance = T::MinXcavBalance::get();
        assert_ok!(T::NativeCurrency::mint_into(&caller, min_balance));
        assert_ok!(T::NativeCurrency::mint_into(&admin, min_balance));

        let asset_id = T::DripAssetId::get();
        let _ = T::ForeignCurrency::create(asset_id, admin, true, 1u128.into());

        #[extrinsic_call]
        drip(RawOrigin::Signed(caller.clone()));

        assert!(LastClaim::<T>::contains_key(&caller));
    }

    impl_benchmark_test_suite!(Faucet, crate::mock::new_test_ext(), crate::mock::Test);
}
