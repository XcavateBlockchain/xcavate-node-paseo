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
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::BadOrigin;

// add_admin tests

#[test]
fn add_admin_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), 1));
        assert_eq!(AdminAccounts::<Test>::get(&1).unwrap(), ());
        // Check if is_admin works as expected.
        assert!(Whitelist::is_admin(&1));
    });
}

#[test]
fn add_admin_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_noop!(Whitelist::add_admin(RuntimeOrigin::signed(2), 1), BadOrigin);
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), 1));
        assert_noop!(Whitelist::add_admin(RuntimeOrigin::root(), 1), Error::<Test>::AlreadyAdmin);
    });
}

// remove_admin tests

#[test]
fn remove_admin_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), 1));
        assert_eq!(AdminAccounts::<Test>::get(&1).unwrap(), ());
        assert_ok!(Whitelist::remove_admin(RuntimeOrigin::root(), 1));
        assert_eq!(AdminAccounts::<Test>::get(&1), None);
        // Check if is_admin works as expected after removal.
        assert!(!Whitelist::is_admin(&1));
    });
}

#[test]
fn remove_admin_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_noop!(Whitelist::remove_admin(RuntimeOrigin::signed(2), 1), BadOrigin);
        assert_noop!(
            Whitelist::remove_admin(RuntimeOrigin::root(), 1),
            Error::<Test>::AccountNotAdmin
        );
    });
}

// assign_role tests

#[test]
fn assign_role_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), 3));
        assert_ok!(Whitelist::assign_role(RuntimeOrigin::signed(3), 1, Role::Lawyer));
        // Check if has_role and is_compliant work as expected.
        assert!(Whitelist::has_role(&1, Role::Lawyer));
        assert!(!Whitelist::has_role(&1, Role::LettingAgent));
        // Check if is_compliant works as expected.
        assert!(Whitelist::is_compliant(&1, Role::Lawyer));
        assert!(!Whitelist::is_compliant(&1, Role::LettingAgent));
        assert_eq!(
            AccountRoles::<Test>::get(&1, Role::Lawyer).unwrap(),
            AccessPermission::Compliant
        );
    });
}

#[test]
fn assign_role_fails_when_user_already_added() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), 3));
        assert_ok!(Whitelist::assign_role(RuntimeOrigin::signed(3), 1, Role::LettingAgent));
        assert_noop!(
            Whitelist::assign_role(RuntimeOrigin::signed(3), 1, Role::LettingAgent),
            Error::<Test>::RoleAlreadyAssigned
        );
    });
}

#[test]
fn assign_role_fails_with_no_permission() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_noop!(
            Whitelist::assign_role(RuntimeOrigin::signed(2), 1, Role::LettingAgent),
            Error::<Test>::AccountNotAdmin
        );
    });
}

// remove_role tests

#[test]
fn remove_role_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), 3));
        assert_ok!(Whitelist::assign_role(RuntimeOrigin::signed(3), 1, Role::RealEstateInvestor));
        assert_ok!(Whitelist::remove_role(RuntimeOrigin::signed(3), 1, Role::RealEstateInvestor));
        // Check if has_role works as expected after removing the role.
        assert!(!Whitelist::has_role(&1, Role::RealEstateInvestor));
        assert!(AccountRoles::<Test>::get(&1, Role::Lawyer).is_none());
    });
}

#[test]
fn remove_role_fails_with_no_permission() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), 3));
        assert_ok!(Whitelist::assign_role(RuntimeOrigin::signed(3), 1, Role::RealEstateInvestor));
        assert_noop!(
            Whitelist::remove_role(RuntimeOrigin::signed(2), 1, Role::RealEstateInvestor),
            Error::<Test>::AccountNotAdmin
        );
    });
}

#[test]
fn remove_role_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), 3));
        assert_noop!(
            Whitelist::remove_role(RuntimeOrigin::signed(3), 1, Role::RealEstateInvestor),
            Error::<Test>::RoleNotAssigned
        );
    });
}

// set_permission tests

#[test]
fn set_permission_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), 3));
        assert_ok!(Whitelist::assign_role(RuntimeOrigin::signed(3), 1, Role::Lawyer));
        assert!(Whitelist::has_role(&1, Role::Lawyer));
        assert_eq!(
            AccountRoles::<Test>::get(&1, Role::Lawyer).unwrap(),
            AccessPermission::Compliant
        );
        assert_ok!(Whitelist::set_permission(
            RuntimeOrigin::signed(3),
            1,
            Role::Lawyer,
            AccessPermission::Revoked
        ));
        assert!(Whitelist::has_role(&1, Role::Lawyer));
        assert_eq!(AccountRoles::<Test>::get(&1, Role::Lawyer).unwrap(), AccessPermission::Revoked);
        assert!(!Whitelist::is_compliant(&1, Role::Lawyer));
        assert_ok!(Whitelist::set_permission(
            RuntimeOrigin::signed(3),
            1,
            Role::Lawyer,
            AccessPermission::Compliant
        ));
        assert_eq!(
            AccountRoles::<Test>::get(&1, Role::Lawyer).unwrap(),
            AccessPermission::Compliant
        );
        assert!(Whitelist::is_compliant(&1, Role::Lawyer));
    });
}

#[test]
fn set_permission_fails() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        assert_ok!(Whitelist::add_admin(RuntimeOrigin::root(), 3));
        assert_ok!(Whitelist::assign_role(RuntimeOrigin::signed(3), 1, Role::Lawyer));
        assert_noop!(
            Whitelist::set_permission(
                RuntimeOrigin::signed(3),
                1,
                Role::LettingAgent,
                AccessPermission::Revoked
            ),
            Error::<Test>::RoleNotAssigned
        );
        assert_ok!(Whitelist::set_permission(
            RuntimeOrigin::signed(3),
            1,
            Role::Lawyer,
            AccessPermission::Revoked
        ));
        assert_noop!(
            Whitelist::set_permission(
                RuntimeOrigin::signed(2),
                1,
                Role::Lawyer,
                AccessPermission::Revoked
            ),
            Error::<Test>::AccountNotAdmin
        );
        assert_noop!(
            Whitelist::set_permission(
                RuntimeOrigin::signed(3),
                1,
                Role::Lawyer,
                AccessPermission::Revoked
            ),
            Error::<Test>::PermissionAlreadySet
        );
    });
}
