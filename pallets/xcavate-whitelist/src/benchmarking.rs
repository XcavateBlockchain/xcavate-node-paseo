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

//! Benchmarking setup for pallet-whitelist
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as Whitelist;
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn add_admin() {
        let user: T::AccountId = account("admin", 0, 0);
        #[extrinsic_call]
        add_admin(RawOrigin::Root, user.clone());

        assert!(AdminAccounts::<T>::contains_key(&user));
    }

    #[benchmark]
    fn remove_admin() {
        let user: T::AccountId = account("admin", 0, 0);

        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), user.clone()));

        assert!(AdminAccounts::<T>::contains_key(&user));

        #[extrinsic_call]
        remove_admin(RawOrigin::Root, user.clone());

        assert!(!AdminAccounts::<T>::contains_key(&user));
    }

    #[benchmark]
    fn assign_role() {
        let admin: T::AccountId = account("admin", 0, 0);
        let user: T::AccountId = account("user", 0, 0);

        T::BenchmarkHelper::setup_airdrop_asset();

        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));
        #[extrinsic_call]
        assign_role(RawOrigin::Signed(admin.clone()), user.clone(), Role::LettingAgent);

        assert_eq!(
            AccountRoles::<T>::get(&user, Role::LettingAgent).unwrap(),
            AccessPermission::Compliant
        );
    }

    #[benchmark]
    fn remove_role() {
        let admin: T::AccountId = account("admin", 0, 0);
        let user: T::AccountId = account("user", 0, 0);

        T::BenchmarkHelper::setup_airdrop_asset();

        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            user.clone(),
            Role::LettingAgent
        ));

        assert!(AccountRoles::<T>::contains_key(&user, Role::LettingAgent));

        #[extrinsic_call]
        remove_role(RawOrigin::Signed(admin.clone()), user.clone(), Role::LettingAgent);

        assert!(!AccountRoles::<T>::contains_key(&user, Role::LettingAgent));
    }

    #[benchmark]
    fn set_permission() {
        let admin: T::AccountId = account("admin", 0, 0);
        let user: T::AccountId = account("user", 0, 0);

        T::BenchmarkHelper::setup_airdrop_asset();

        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            user.clone(),
            Role::LettingAgent
        ));

        assert_eq!(
            AccountRoles::<T>::get(&user, Role::LettingAgent).unwrap(),
            AccessPermission::Compliant
        );

        #[extrinsic_call]
        set_permission(
            RawOrigin::Signed(admin.clone()),
            user.clone(),
            Role::LettingAgent,
            AccessPermission::Revoked,
        );

        assert_eq!(
            AccountRoles::<T>::get(&user, Role::LettingAgent).unwrap(),
            AccessPermission::Revoked
        );
    }

    impl_benchmark_test_suite!(Whitelist, crate::mock::new_test_ext(), crate::mock::Test);
}
