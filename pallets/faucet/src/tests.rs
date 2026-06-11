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

use crate::{mock::*, Error, LastClaim};
use frame_support::{assert_noop, assert_ok};

#[test]
fn drip_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Faucet::drip(RuntimeOrigin::signed([1; 32].into())));
        assert_eq!(ForeignAssets::balance(10, &([1; 32].into())), DripAmount::get());
        assert_eq!(LastClaim::<Test>::get(&AccountId::from([1; 32])), Some(1));
    });
}

#[test]
fn drip_works_at_exact_minimum_balance() {
    new_test_ext().execute_with(|| {
        assert_ok!(Faucet::drip(RuntimeOrigin::signed([3; 32].into())));
        assert_eq!(ForeignAssets::balance(10, &([3; 32].into())), DripAmount::get());
    });
}

#[test]
fn drip_fails_insufficient_xcav() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Faucet::drip(RuntimeOrigin::signed([2; 32].into())),
            Error::<Test>::InsufficientXcavBalance
        );
    });
}

#[test]
fn drip_fails_cooldown_not_elapsed() {
    new_test_ext().execute_with(|| {
        assert_ok!(Faucet::drip(RuntimeOrigin::signed([1; 32].into())));

        // Immediately try again — should fail.
        assert_noop!(
            Faucet::drip(RuntimeOrigin::signed([1; 32].into())),
            Error::<Test>::CooldownNotElapsed
        );

        // Advance but still within cooldown (claimed at block 1, cooldown is DAYS).
        System::set_block_number(DAYS);
        assert_noop!(
            Faucet::drip(RuntimeOrigin::signed([1; 32].into())),
            Error::<Test>::CooldownNotElapsed
        );
    });
}

#[test]
fn drip_works_after_cooldown() {
    new_test_ext().execute_with(|| {
        assert_ok!(Faucet::drip(RuntimeOrigin::signed([1; 32].into())));
        assert_eq!(ForeignAssets::balance(10, &([1; 32].into())), DripAmount::get());

        // Advance past cooldown.
        System::set_block_number(1 + DAYS);
        assert_ok!(Faucet::drip(RuntimeOrigin::signed([1; 32].into())));
        assert_eq!(ForeignAssets::balance(10, &([1; 32].into())), DripAmount::get() * 2);
        assert_eq!(LastClaim::<Test>::get(&AccountId::from([1; 32])), Some(1 + DAYS));
    });
}

#[test]
fn drip_fails_for_unknown_account() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Faucet::drip(RuntimeOrigin::signed([99; 32].into())),
            Error::<Test>::InsufficientXcavBalance
        );
    });
}
