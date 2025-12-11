//! Migration to create reserved assets in pallet-assets (Instance2).
//!
//! Creates:
//! - Asset 0: XCAV (Native token representation)
//! - Asset 1: tGBP (Bridged stablecoin)
//!
//! This migration is idempotent - it checks if assets exist before creating them.

use frame_support::{
    storage::migration::get_storage_value, traits::OnRuntimeUpgrade, weights::Weight, BoundedVec,
};
use pallet_assets::Instance2;
use sp_core::crypto::Ss58Codec;
use sp_runtime::traits::Zero;
#[cfg(feature = "try-runtime")]
use {
    alloc::vec::Vec,
    frame_support::ensure,
    parity_scale_codec::{Decode, Encode},
    sp_runtime::TryRuntimeError,
};

use crate::{
    constants::{
        currency::MILLIXCAV,
        known_assets::{NATIVE_ASSET_ID, TGBP_ASSET_ID},
    },
    AccountId, Balance, Runtime,
};

const LOG_TARGET: &str = "runtime::migrations::reserved_assets";

/// Minimum balance for tGBP (18 decimals, 1 unit = 10^18)
const TGBP_MIN_BALANCE: Balance = 1_000_000_000_000_000_000;

/// Expected sudo key (SS58 format) for this migration.
///
/// ⚠️  IMPORTANT: Update this value before deploying to a new environment!
///
/// This is a safety check to ensure the migration runs with the expected sudo key.
/// The sudo account will become the owner/admin of the reserved assets.
const EXPECTED_SUDO_KEY_SS58: &str = "12DvowjMsZ8cVwnZCiUEMDNUAuZgMErxRx7LAb5f85maiCT4";

/// Migration that creates reserved assets (XCAV at ID 0, tGBP at ID 1).
///
/// The assets are created with the sudo account as owner/admin/issuer/freezer.
/// This migration is idempotent - if assets already exist, it skips creation.
pub struct CreateReservedAssets;

impl OnRuntimeUpgrade for CreateReservedAssets {
    fn on_runtime_upgrade() -> Weight {
        let mut reads = 0u64;
        let mut writes = 0u64;

        // Get sudo key - required for asset ownership.
        let sudo_key: Option<AccountId> = get_storage_value(b"Sudo", b"Key", &[]);
        let sudo_key = match sudo_key {
            Some(key) => key,
            None => {
                log::error!(
                    target: LOG_TARGET,
                    "Sudo key not set - cannot create reserved assets"
                );
                return Weight::zero();
            }
        };
        reads += 1;

        // Verify sudo key matches expected value
        let expected_sudo_key = match AccountId::from_ss58check(EXPECTED_SUDO_KEY_SS58) {
            Ok(key) => key,
            Err(e) => {
                log::error!(
                    target: LOG_TARGET,
                    "Invalid EXPECTED_SUDO_KEY_SS58 constant: {:?}",
                    e
                );
                return Weight::zero();
            }
        };

        if sudo_key != expected_sudo_key {
            log::error!(
                target: LOG_TARGET,
                "Sudo key mismatch! Expected {:?}, got {:?}. \
                Update EXPECTED_SUDO_KEY_SS58 in migrations/reserved_assets.rs",
                expected_sudo_key,
                sudo_key
            );
            return Weight::zero();
        }

        log::info!(
            target: LOG_TARGET,
            "Sudo key verified: {}",
            EXPECTED_SUDO_KEY_SS58
        );

        // Create Asset 0: XCAV
        if !pallet_assets::Asset::<Runtime, Instance2>::contains_key(NATIVE_ASSET_ID) {
            create_asset(NATIVE_ASSET_ID, &sudo_key, MILLIXCAV, b"Xcavate", b"XCAV", 12);
            writes += 2; // Asset + Metadata
            log::info!(
                target: LOG_TARGET,
                "Created XCAV asset (ID={})",
                NATIVE_ASSET_ID
            );
        } else {
            log::warn!(
                target: LOG_TARGET,
                "XCAV asset (ID={}) already exists, skipping",
                NATIVE_ASSET_ID
            );
        }
        reads += 1;

        // Create Asset 1: tGBP
        if !pallet_assets::Asset::<Runtime, Instance2>::contains_key(TGBP_ASSET_ID) {
            create_asset(TGBP_ASSET_ID, &sudo_key, TGBP_MIN_BALANCE, b"Tokenised GBP", b"tGBP", 18);
            writes += 2; // Asset + Metadata
            log::info!(
                target: LOG_TARGET,
                "Created tGBP asset (ID={})",
                TGBP_ASSET_ID
            );
        } else {
            log::warn!(
                target: LOG_TARGET,
                "tGBP asset (ID={}) already exists, skipping",
                TGBP_ASSET_ID
            );
        }
        reads += 1;

        log::info!(
            target: LOG_TARGET,
            "Migration complete: {} reads, {} writes",
            reads,
            writes
        );

        <Runtime as frame_system::Config>::DbWeight::get().reads_writes(reads, writes)
    }

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
        let xcav_exists = pallet_assets::Asset::<Runtime, Instance2>::contains_key(NATIVE_ASSET_ID);
        let tgbp_exists = pallet_assets::Asset::<Runtime, Instance2>::contains_key(TGBP_ASSET_ID);

        log::info!(
            target: LOG_TARGET,
            "pre_upgrade: XCAV exists={}, tGBP exists={}",
            xcav_exists,
            tgbp_exists
        );

        Ok((xcav_exists, tgbp_exists).encode())
    }

    #[cfg(feature = "try-runtime")]
    fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
        let (xcav_existed, tgbp_existed): (bool, bool) =
            Decode::decode(&mut &state[..]).map_err(|_| "Failed to decode pre_upgrade state")?;

        // XCAV should exist after migration
        ensure!(
            pallet_assets::Asset::<Runtime, Instance2>::contains_key(NATIVE_ASSET_ID),
            "XCAV asset not created"
        );

        // tGBP should exist after migration
        ensure!(
            pallet_assets::Asset::<Runtime, Instance2>::contains_key(TGBP_ASSET_ID),
            "tGBP asset not created"
        );

        // Verify XCAV details if newly created
        if !xcav_existed {
            let xcav = pallet_assets::Asset::<Runtime, Instance2>::get(NATIVE_ASSET_ID)
                .ok_or("XCAV asset details missing")?;
            ensure!(xcav.min_balance == MILLIXCAV, "XCAV min_balance incorrect");
            ensure!(xcav.status == pallet_assets::AssetStatus::Live, "XCAV not Live");

            let meta = pallet_assets::Metadata::<Runtime, Instance2>::get(NATIVE_ASSET_ID);
            ensure!(meta.decimals == 12, "XCAV decimals incorrect");
            ensure!(meta.symbol.as_slice() == b"XCAV", "XCAV symbol incorrect");
        }

        // Verify tGBP details if newly created
        if !tgbp_existed {
            let tgbp = pallet_assets::Asset::<Runtime, Instance2>::get(TGBP_ASSET_ID)
                .ok_or("tGBP asset details missing")?;
            ensure!(tgbp.min_balance == TGBP_MIN_BALANCE, "tGBP min_balance incorrect");
            ensure!(tgbp.status == pallet_assets::AssetStatus::Live, "tGBP not Live");

            let meta = pallet_assets::Metadata::<Runtime, Instance2>::get(TGBP_ASSET_ID);
            ensure!(meta.decimals == 18, "tGBP decimals incorrect");
            ensure!(meta.symbol.as_slice() == b"tGBP", "tGBP symbol incorrect");
        }

        log::info!(target: LOG_TARGET, "post_upgrade checks passed");
        Ok(())
    }
}

/// Helper function to create an asset and its metadata.
fn create_asset(
    asset_id: u32,
    owner: &AccountId,
    min_balance: Balance,
    name: &[u8],
    symbol: &[u8],
    decimals: u8,
) {
    type StringLimit = <Runtime as pallet_assets::Config<Instance2>>::StringLimit;

    let asset_details = pallet_assets::AssetDetails {
        owner: owner.clone(),
        issuer: owner.clone(),
        admin: owner.clone(),
        freezer: owner.clone(),
        supply: Zero::zero(),
        deposit: Zero::zero(),
        min_balance,
        is_sufficient: false,
        accounts: 0,
        sufficients: 0,
        approvals: 0,
        status: pallet_assets::AssetStatus::Live,
    };

    let bounded_name: BoundedVec<u8, StringLimit> =
        name.to_vec().try_into().expect("asset name too long");
    let bounded_symbol: BoundedVec<u8, StringLimit> =
        symbol.to_vec().try_into().expect("asset symbol too long");

    let metadata = pallet_assets::AssetMetadata {
        deposit: Zero::zero(),
        name: bounded_name,
        symbol: bounded_symbol,
        decimals,
        is_frozen: false,
    };

    pallet_assets::Asset::<Runtime, Instance2>::insert(asset_id, asset_details);
    pallet_assets::Metadata::<Runtime, Instance2>::insert(asset_id, metadata);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tgbp_min_balance_is_one_unit() {
        // 1 unit with 18 decimals = 10^18
        assert_eq!(TGBP_MIN_BALANCE, 1_000_000_000_000_000_000);
    }

    #[test]
    fn xcav_min_balance_matches_native_ed() {
        // MILLIXCAV = 1_000_000_000 (native ED)
        assert_eq!(MILLIXCAV, 1_000_000_000);
    }
}
