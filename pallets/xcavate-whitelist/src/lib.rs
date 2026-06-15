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
    traits::{fungible, fungibles::Mutate as FungiblesMutate, tokens::Balance},
};

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[cfg(feature = "runtime-benchmarks")]
pub trait BenchmarkHelper<T: pallet::Config> {
    fn setup_airdrop_asset();
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::traits::fungible::Mutate as FungibleMutate;
    use frame_system::pallet_prelude::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Enum representing all possible roles in the Xcavate ecosystem.
    /// Each role has a specific function in real estate operations.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(
        Encode,
        Decode,
        DecodeWithMemTracking,
        Clone,
        PartialEq,
        Eq,
        MaxEncodedLen,
        RuntimeDebug,
        TypeInfo,
    )]
    pub enum Role {
        /// Manages regional operations and oversees real estate activities in a region.
        RegionalOperator,
        /// Invests in real estate assets.
        RealEstateInvestor,
        /// Develops and sells real estate projects, creating new property assets.
        RealEstateDeveloper,
        /// Handles legal processes related to real estate transactions and contracts.
        Lawyer,
        /// Manages rental properties and distributes rental income to investors.
        LettingAgent,
        /// Confirms the setup and compliance of Special Purpose Vehicles (SPVs).
        SpvConfirmation,
        /// Creates educational content/modules that can be sponsored and delivered.
        ModuleCreator,
        /// Funds the delivery of educational modules.
        ModuleSponsor,
        /// Books a sponsored module for delivery to students.
        ModuleBooker,
        /// Delivers the educational module (lecturer / teacher).
        ModuleDeliverer,
        /// AI agent responsible for evaluating impact / scores.
        ModuleAIAgent,
        /// Recipient of the educational content.
        ModuleRecipient,
    }

    /// Defines the compliant status of users in the Xcavate ecosystem.
    /// The compliant status is managed by whitelist admins.
    #[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
    #[derive(
        Encode,
        Decode,
        DecodeWithMemTracking,
        Clone,
        PartialEq,
        Eq,
        MaxEncodedLen,
        RuntimeDebug,
        TypeInfo,
    )]
    pub enum AccessPermission {
        /// A users compliant status can be set to revoked if he does not meet the legal requirements anymore,
        /// blocking role specific actions.
        Revoked,
        /// A users compliant status is set by default to compliant after passing the KYC/AML,
        /// allowing role specific actions.
        Compliant,
    }

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Type representing the weight of this pallet.
        type WeightInfo: WeightInfo;
        /// Origin required to manage whitelist admins.
        type WhitelistOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// The balance type.
        type Balance: Balance + TypeInfo + From<u128> + Default;

        /// The native currency for minting XCAV on role assignment.
        type NativeCurrency: fungible::Mutate<AccountIdOf<Self>, Balance = Self::Balance>;

        /// The foreign assets pallet for minting tGBP on role assignment.
        type ForeignCurrency: FungiblesMutate<
            AccountIdOf<Self>,
            AssetId = u32,
            Balance = Self::Balance,
        >;

        #[cfg(feature = "runtime-benchmarks")]
        type BenchmarkHelper: BenchmarkHelper<Self>;

        /// Amount of native XCAV to mint on role assignment.
        #[pallet::constant]
        type AirdropNativeAmount: Get<Self::Balance>;

        /// Asset ID for testnet tGBP airdrop.
        #[pallet::constant]
        type AirdropAssetId: Get<u32>;

        /// Amount of tGBP to mint on role assignment.
        #[pallet::constant]
        type AirdropAssetAmount: Get<Self::Balance>;
    }

    /// Mapping of the admin accounts.
    #[pallet::storage]
    pub type AdminAccounts<T: Config> =
        StorageMap<_, Blake2_128Concat, AccountIdOf<T>, (), OptionQuery>;

    /// Mapping of accounts to their assigned roles and permissions.
    #[pallet::storage]
    pub type AccountRoles<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        AccountIdOf<T>,
        Blake2_128Concat,
        Role,
        AccessPermission,
        OptionQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new role has been assigned to a user.
        RoleAssigned { user: T::AccountId, role: Role },
        /// A role has been removed from a user.
        RoleRemoved { user: T::AccountId, role: Role },
        /// A new admin has been added.
        AdminRegistered { admin: T::AccountId },
        /// An admin has been removed.
        AdminRemoved { admin: T::AccountId },
        /// A user’s compliance status has been updated.
        PermissionUpdated { user: T::AccountId, role: Role, permission: AccessPermission },
        /// Tokens were airdropped to a user on role assignment.
        Airdropped {
            user: T::AccountId,
            native_amount: T::Balance,
            asset_id: u32,
            asset_amount: T::Balance,
        },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// The role has already been assigned to the user.
        RoleAlreadyAssigned,
        /// The role has not been assigned to the user.
        RoleNotAssigned,
        /// The account is already registered as an admin.
        AlreadyAdmin,
        /// The account is not registered as an admin.
        AccountNotAdmin,
        /// This permission has already been set.
        PermissionAlreadySet,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Add a new whitelist admin.
        ///
        /// The origin must be the sudo.
        ///
        /// Parameters:
        /// - `admin`: The address of the account to add as an admin.
        ///
        /// Emits `AdminRegistered` event when successful.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::add_admin())]
        pub fn add_admin(origin: OriginFor<T>, admin: AccountIdOf<T>) -> DispatchResult {
            T::WhitelistOrigin::ensure_origin(origin)?;
            // Prevent double-registration.
            ensure!(!AdminAccounts::<T>::contains_key(&admin), Error::<T>::AlreadyAdmin);
            AdminAccounts::<T>::insert(&admin, ());
            Self::deposit_event(Event::<T>::AdminRegistered { admin });
            Ok(())
        }

        /// Remove an existing whitelist admin.
        ///
        /// The origin must be the sudo.
        ///
        /// Parameters:
        /// - `admin`: The address of the account to remove as an admin.
        ///
        /// Emits `AdminRemoved` event when successful.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::remove_admin())]
        pub fn remove_admin(origin: OriginFor<T>, admin: AccountIdOf<T>) -> DispatchResult {
            T::WhitelistOrigin::ensure_origin(origin)?;
            ensure!(AdminAccounts::<T>::contains_key(&admin), Error::<T>::AccountNotAdmin);
            AdminAccounts::<T>::remove(&admin);
            Self::deposit_event(Event::<T>::AdminRemoved { admin });
            Ok(())
        }

        /// Assign a role to a user with default 'Compliant' permission.
        ///
        /// The origin must be an admin.
        ///
        /// Parameters:
        /// - `user`: The address of the account that gets a new role.
        /// - `role`: The role that is getting assigned to the user.
        ///
        /// Emits `RoleAssigned` event when successful.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::assign_role())]
        pub fn assign_role(
            origin: OriginFor<T>,
            user: AccountIdOf<T>,
            role: Role,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            // Verify that the caller is a registered admin.
            ensure!(AdminAccounts::<T>::contains_key(&signer), Error::<T>::AccountNotAdmin);
            // Avoid duplicate role assignments.
            ensure!(
                !AccountRoles::<T>::contains_key(&user, &role),
                Error::<T>::RoleAlreadyAssigned
            );
            AccountRoles::<T>::insert(&user, role.clone(), AccessPermission::Compliant);

            let native_amount = T::AirdropNativeAmount::get();
            let asset_id = T::AirdropAssetId::get();
            let asset_amount = T::AirdropAssetAmount::get();

            let native_ok = T::NativeCurrency::mint_into(&user, native_amount).is_ok();
            let asset_ok = T::ForeignCurrency::mint_into(asset_id, &user, asset_amount).is_ok();

            if native_ok || asset_ok {
                Self::deposit_event(Event::<T>::Airdropped {
                    user: user.clone(),
                    native_amount,
                    asset_id,
                    asset_amount,
                });
            }
            Self::deposit_event(Event::<T>::RoleAssigned { user, role });
            Ok(())
        }

        /// Remove a role from a user.
        ///
        /// The origin must be an admin.
        ///
        /// Parameters:
        /// - `user`: The address of the account that gets a role removed.
        /// - `role`: The role that is getting removed from the user.
        ///
        /// Emits `RoleRemoved` event when successful.
        #[pallet::call_index(3)]
        #[pallet::weight((T::WeightInfo::remove_role(), DispatchClass::Operational))]
        pub fn remove_role(
            origin: OriginFor<T>,
            user: AccountIdOf<T>,
            role: Role,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            // Verify that the caller is a registered admin.
            ensure!(AdminAccounts::<T>::contains_key(&signer), Error::<T>::AccountNotAdmin);
            ensure!(AccountRoles::<T>::contains_key(&user, &role), Error::<T>::RoleNotAssigned);
            AccountRoles::<T>::remove(&user, role.clone());
            Self::deposit_event(Event::<T>::RoleRemoved { user, role });
            Ok(())
        }

        /// Update a user's permission for a role.
        ///
        /// The origin must be an admin.
        ///
        /// Parameters:
        /// - `user`: The address of the account that gets the permission updated.
        /// - `role`: The role that is getting the permission updated.
        /// - `permission`: The new permission state.
        ///
        /// Emits `PermissionUpdated` event when successful.
        #[pallet::call_index(4)]
        #[pallet::weight((T::WeightInfo::set_permission(), DispatchClass::Operational))]
        pub fn set_permission(
            origin: OriginFor<T>,
            user: AccountIdOf<T>,
            role: Role,
            permission: AccessPermission,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            // Verify that the caller is a registered admin.
            ensure!(AdminAccounts::<T>::contains_key(&signer), Error::<T>::AccountNotAdmin);
            ensure!(
                AccountRoles::<T>::get(&user, &role).ok_or(Error::<T>::RoleNotAssigned)?
                    != permission,
                Error::<T>::PermissionAlreadySet
            );

            AccountRoles::<T>::insert(&user, role.clone(), permission.clone());
            Self::deposit_event(Event::<T>::PermissionUpdated { user, role, permission });
            Ok(())
        }
    }
}

/// Trait for checking account roles and compliance in the Xcavate ecosystem.
pub trait RolePermission<AccountId> {
    /// Checks if an account has a specific role (e.g., `LettingAgent`).
    ///
    /// Returns `true` if the account has the role, regardless of compliance.
    fn has_role(account: &AccountId, role: Role) -> bool;

    /// Checks if an account has a role and is compliant for that role.
    ///
    /// Returns `true` if the account’s role is `Compliant`, ensuring KYC/AML validity.
    fn is_compliant(account: &AccountId, role: Role) -> bool;

    /// Checks if an account is a whitelist admin.
    ///
    /// Returns `true` if the account is an admin, allowing actions like `assign_role`.
    fn is_admin(account: &AccountId) -> bool;
}

/// Trait for removing a Role from an account.
pub trait RoleRemover<AccountId> {
    /// Remove a role from an account.
    fn role_removal(account: AccountId, role: Role) -> DispatchResult;
}

impl<T: Config> RolePermission<T::AccountId> for Pallet<T> {
    fn has_role(account: &T::AccountId, role: Role) -> bool {
        AccountRoles::<T>::contains_key(account, role)
    }

    fn is_compliant(account: &T::AccountId, role: Role) -> bool {
        AccountRoles::<T>::get(account, role) == Some(AccessPermission::Compliant)
    }

    fn is_admin(account: &T::AccountId) -> bool {
        AdminAccounts::<T>::contains_key(account)
    }
}

impl<T: Config> RoleRemover<T::AccountId> for Pallet<T> {
    fn role_removal(account: T::AccountId, role: Role) -> DispatchResult {
        ensure!(AccountRoles::<T>::contains_key(&account, &role), Error::<T>::RoleNotAssigned);
        AccountRoles::<T>::remove(&account, role.clone());
        Self::deposit_event(Event::<T>::RoleRemoved { user: account, role });
        Ok(())
    }
}
