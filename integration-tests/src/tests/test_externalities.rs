//! Test externalities setup for runtime integration tests

use frame_support::traits::Get;
use sp_io::TestExternalities;
use sp_runtime::BuildStorage;
use xcavate_runtime::{
    configs::ismp::AssetAdmin, constants::currency::XCAV, Runtime, RuntimeOrigin, System,
};

/// Creates a new test externalities with default genesis configuration
pub fn new_test_ext() -> TestExternalities {
    let mut storage = frame_system::GenesisConfig::<Runtime>::default()
        .build_storage()
        .expect("Failed to create test externalities");

    // Fund treasury. Necessary for asset creation.
    let treasury = AssetAdmin::get();
    pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![(treasury, 1000 * XCAV)],
        dev_accounts: None,
    }
    .assimilate_storage(&mut storage)
    .expect("Failed to assimilate balances");

    let mut ext: TestExternalities = storage.into();
    ext.execute_with(|| {
        // Set block number to 1 (block 0 has special behavior)
        System::set_block_number(1);
    });

    ext
}

/// Helper to get the root origin for privileged operations
pub fn root_origin() -> RuntimeOrigin {
    RuntimeOrigin::root()
}

/// Helper to get a signed origin for a specific account
pub fn signed_origin(who: sp_runtime::AccountId32) -> RuntimeOrigin {
    RuntimeOrigin::signed(who)
}
