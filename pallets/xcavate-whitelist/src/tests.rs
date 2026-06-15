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

use crate::{mock::*, AccessPermission, AccountRoles, AdminAccounts, Error, Role, RolePermission};
use frame_support::{assert_noop, assert_ok, traits::fungible::Inspect};
use sp_runtime::traits::BadOrigin;

// add_admin tests

#[test]
fn add_admin_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let account_1: AccountId = [1; 32].into();
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), account_1.clone()));
        assert_eq!(AdminAccounts::<Test>::get(&account_1).unwrap(), ());
        assert!(Whitelist::is_admin(&account_1));
    });
}

#[test]
fn add_admin_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let account_1: AccountId = [1; 32].into();
        let account_2: AccountId = [2; 32].into();
        assert_noop!(
            Whitelist::add_admin(RuntimeOrigin::signed(account_2), account_1.clone()),
            BadOrigin
        );
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), account_1.clone()));
        assert_noop!(
            Whitelist::add_admin(RuntimeOrigin::root(), account_1),
            Error::<Test>::AlreadyAdmin
        );
    });
}

// remove_admin tests

#[test]
fn remove_admin_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let account_1: AccountId = [1; 32].into();
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), account_1.clone()));
        assert_eq!(AdminAccounts::<Test>::get(&account_1).unwrap(), ());
        assert_ok!(Whitelist::remove_admin(RuntimeOrigin::root(), account_1.clone()));
        assert_eq!(AdminAccounts::<Test>::get(&account_1), None);
        assert!(!Whitelist::is_admin(&account_1));
    });
}

#[test]
fn remove_admin_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let account_1: AccountId = [1; 32].into();
        let account_2: AccountId = [2; 32].into();
        assert_noop!(
            Whitelist::remove_admin(RuntimeOrigin::signed(account_2), account_1.clone()),
            BadOrigin
        );
        assert_noop!(
            Whitelist::remove_admin(RuntimeOrigin::root(), account_1),
            Error::<Test>::AccountNotAdmin
        );
    });
}

// assign_role tests

#[test]
fn assign_role_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let admin: AccountId = [3; 32].into();
        let user: AccountId = [1; 32].into();
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), admin.clone()));
        assert_ok!(Whitelist::assign_role(
            RuntimeOrigin::signed(admin),
            user.clone(),
            Role::Lawyer
        ));
        assert!(Whitelist::has_role(&user, Role::Lawyer));
        assert!(!Whitelist::has_role(&user, Role::LettingAgent));
        assert!(Whitelist::is_compliant(&user, Role::Lawyer));
        assert!(!Whitelist::is_compliant(&user, Role::LettingAgent));
        assert_eq!(
            AccountRoles::<Test>::get(&user, Role::Lawyer).unwrap(),
            AccessPermission::Compliant
        );
    });
}

#[test]
fn assign_role_mints_airdrop() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let admin: AccountId = [3; 32].into();
        let user: AccountId = [1; 32].into();
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), admin.clone()));
        assert_ok!(Whitelist::assign_role(
            RuntimeOrigin::signed(admin),
            user.clone(),
            Role::RealEstateInvestor
        ));

        // 10 XCAV (12 decimals)
        assert_eq!(Balances::balance(&user), 10_000_000_000_000);
        // 10,000 tGBP (18 decimals) on asset ID 10
        assert_eq!(ForeignAssets::balance(10, &user), 10_000_000_000_000_000_000_000);
    });
}

#[test]
fn assign_multiple_roles_gives_multiple_airdrops() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let admin: AccountId = [3; 32].into();
        let user: AccountId = [1; 32].into();
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), admin.clone()));
        assert_ok!(Whitelist::assign_role(
            RuntimeOrigin::signed(admin.clone()),
            user.clone(),
            Role::RealEstateInvestor
        ));
        assert_ok!(Whitelist::assign_role(
            RuntimeOrigin::signed(admin),
            user.clone(),
            Role::Lawyer
        ));

        // Two role assignments = 2x airdrop
        assert_eq!(Balances::balance(&user), 2 * 10_000_000_000_000);
        assert_eq!(ForeignAssets::balance(10, &user), 2 * 10_000_000_000_000_000_000_000);
    });
}

#[test]
fn assign_role_works_without_asset() {
    new_test_ext_no_asset().execute_with(|| {
        System::set_block_number(1);
        let admin: AccountId = [3; 32].into();
        let user: AccountId = [1; 32].into();
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), admin.clone()));
        assert_ok!(Whitelist::assign_role(
            RuntimeOrigin::signed(admin),
            user.clone(),
            Role::RealEstateInvestor
        ));

        assert!(Whitelist::has_role(&user, Role::RealEstateInvestor));
        assert!(Whitelist::is_compliant(&user, Role::RealEstateInvestor));
        // Native mint still works (no asset dependency)
        assert_eq!(Balances::balance(&user), 10_000_000_000_000);
        // Asset mint silently fails — balance is zero
        assert_eq!(ForeignAssets::balance(10, &user), 0);
    });
}

#[test]
fn assign_role_fails_when_user_already_added() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let admin: AccountId = [3; 32].into();
        let user: AccountId = [1; 32].into();
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), admin.clone()));
        assert_ok!(Whitelist::assign_role(
            RuntimeOrigin::signed(admin.clone()),
            user.clone(),
            Role::LettingAgent
        ));
        assert_noop!(
            Whitelist::assign_role(RuntimeOrigin::signed(admin), user, Role::LettingAgent),
            Error::<Test>::RoleAlreadyAssigned
        );
    });
}

#[test]
fn assign_role_fails_with_no_permission() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let user: AccountId = [1; 32].into();
        let non_admin: AccountId = [2; 32].into();
        assert_noop!(
            Whitelist::assign_role(RuntimeOrigin::signed(non_admin), user, Role::LettingAgent),
            Error::<Test>::AccountNotAdmin
        );
    });
}

// remove_role tests

#[test]
fn remove_role_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let admin: AccountId = [3; 32].into();
        let user: AccountId = [1; 32].into();
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), admin.clone()));
        assert_ok!(Whitelist::assign_role(
            RuntimeOrigin::signed(admin.clone()),
            user.clone(),
            Role::RealEstateInvestor
        ));
        assert_ok!(Whitelist::remove_role(
            RuntimeOrigin::signed(admin),
            user.clone(),
            Role::RealEstateInvestor
        ));
        assert!(!Whitelist::has_role(&user, Role::RealEstateInvestor));
        assert!(AccountRoles::<Test>::get(&user, Role::Lawyer).is_none());
    });
}

#[test]
fn remove_role_fails_with_no_permission() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let admin: AccountId = [3; 32].into();
        let user: AccountId = [1; 32].into();
        let non_admin: AccountId = [2; 32].into();
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), admin.clone()));
        assert_ok!(Whitelist::assign_role(
            RuntimeOrigin::signed(admin),
            user.clone(),
            Role::RealEstateInvestor
        ));
        assert_noop!(
            Whitelist::remove_role(
                RuntimeOrigin::signed(non_admin),
                user,
                Role::RealEstateInvestor
            ),
            Error::<Test>::AccountNotAdmin
        );
    });
}

#[test]
fn remove_role_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let admin: AccountId = [3; 32].into();
        let user: AccountId = [1; 32].into();
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), admin.clone()));
        assert_noop!(
            Whitelist::remove_role(RuntimeOrigin::signed(admin), user, Role::RealEstateInvestor),
            Error::<Test>::RoleNotAssigned
        );
    });
}

// set_permission tests

#[test]
fn set_permission_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let admin: AccountId = [3; 32].into();
        let user: AccountId = [1; 32].into();
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), admin.clone()));
        assert_ok!(Whitelist::assign_role(
            RuntimeOrigin::signed(admin.clone()),
            user.clone(),
            Role::Lawyer
        ));
        assert!(Whitelist::has_role(&user, Role::Lawyer));
        assert_eq!(
            AccountRoles::<Test>::get(&user, Role::Lawyer).unwrap(),
            AccessPermission::Compliant
        );
        assert_ok!(Whitelist::set_permission(
            RuntimeOrigin::signed(admin.clone()),
            user.clone(),
            Role::Lawyer,
            AccessPermission::Revoked
        ));
        assert!(Whitelist::has_role(&user, Role::Lawyer));
        assert_eq!(
            AccountRoles::<Test>::get(&user, Role::Lawyer).unwrap(),
            AccessPermission::Revoked
        );
        assert!(!Whitelist::is_compliant(&user, Role::Lawyer));
        assert_ok!(Whitelist::set_permission(
            RuntimeOrigin::signed(admin),
            user.clone(),
            Role::Lawyer,
            AccessPermission::Compliant
        ));
        assert_eq!(
            AccountRoles::<Test>::get(&user, Role::Lawyer).unwrap(),
            AccessPermission::Compliant
        );
        assert!(Whitelist::is_compliant(&user, Role::Lawyer));
    });
}

#[test]
fn set_permission_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let admin: AccountId = [3; 32].into();
        let user: AccountId = [1; 32].into();
        let non_admin: AccountId = [2; 32].into();
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), admin.clone()));
        assert_ok!(Whitelist::assign_role(
            RuntimeOrigin::signed(admin.clone()),
            user.clone(),
            Role::Lawyer
        ));
        assert_noop!(
            Whitelist::set_permission(
                RuntimeOrigin::signed(admin.clone()),
                user.clone(),
                Role::LettingAgent,
                AccessPermission::Revoked
            ),
            Error::<Test>::RoleNotAssigned
        );
        assert_ok!(Whitelist::set_permission(
            RuntimeOrigin::signed(admin.clone()),
            user.clone(),
            Role::Lawyer,
            AccessPermission::Revoked
        ));
        assert_noop!(
            Whitelist::set_permission(
                RuntimeOrigin::signed(non_admin),
                user.clone(),
                Role::Lawyer,
                AccessPermission::Revoked
            ),
            Error::<Test>::AccountNotAdmin
        );
        assert_noop!(
            Whitelist::set_permission(
                RuntimeOrigin::signed(admin),
                user,
                Role::Lawyer,
                AccessPermission::Revoked
            ),
            Error::<Test>::PermissionAlreadySet
        );
    });
}
