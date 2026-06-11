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

use crate as pallet_faucet;
use frame_support::{derive_impl, parameter_types};
use sp_core::ConstU128;
use sp_runtime::{
    traits::{AccountIdLookup, BlakeTwo256, IdentifyAccount, Verify},
    BuildStorage, MultiSignature,
};

use pallet_assets::Instance2;

pub type Block = frame_system::mocking::MockBlock<Test>;

pub type Balance = u128;

pub type BlockNumber = u64;

pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Signature = MultiSignature;

pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        ForeignAssets: pallet_assets::<Instance2>,
        Faucet: pallet_faucet,
    }
);

parameter_types! {
    pub const BlockHashCount: BlockNumber = 2400;
}

#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
    type RuntimeCall = RuntimeCall;
    type Nonce = u32;
    type Block = Block;
    type Hash = sp_core::H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = AccountIdLookup<AccountId, ()>;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u128>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = ();
    type BaseCallFilter = frame_support::traits::Everything;
    type SystemWeightInfo = ();
    type BlockWeights = ();
    type BlockLength = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
    type RuntimeTask = ();
}

impl pallet_balances::Config for Test {
    type Balance = u128;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ConstU128<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ();
    type DoneSlashHandler = ();
}

parameter_types! {
    pub RootAccountId: AccountId = AccountId::from([0xffu8; 32]);
}

impl pallet_assets::Config<Instance2> for Test {
    type RuntimeEvent = RuntimeEvent;
    type Balance = u128;
    type AssetId = u32;
    type AssetIdParameter = parity_scale_codec::Compact<u32>;
    type Currency = Balances;
    type CreateOrigin = frame_support::traits::AsEnsureOriginWithArg<
        frame_system::EnsureRootWithSuccess<AccountId, RootAccountId>,
    >;
    type ForceOrigin = frame_system::EnsureRoot<AccountId>;
    type AssetDeposit = ConstU128<0>;
    type AssetAccountDeposit = ConstU128<0>;
    type MetadataDepositBase = ConstU128<0>;
    type MetadataDepositPerByte = ConstU128<0>;
    type ApprovalDeposit = ConstU128<0>;
    type StringLimit = frame_support::traits::ConstU32<50>;
    type Freezer = ();
    type Holder = ();
    type Extra = ();
    type CallbackHandle = ();
    type WeightInfo = ();
    type RemoveItemsLimit = frame_support::traits::ConstU32<1000>;
}

parameter_types! {
    pub const DripAssetId: u32 = 10;
    pub const DripAmount: Balance = 1_000_000_000_000_000_000_000; // 1000 tGBP (18 decimals)
    pub const MinXcavBalance: Balance = 1_000_000_000_000; // 1 XCAV (12 decimals)
    pub const CooldownPeriod: BlockNumber = DAYS;
}

impl pallet_faucet::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_faucet::weights::SubstrateWeight<Test>;
    type Balance = u128;
    type NativeCurrency = Balances;
    type ForeignCurrency = ForeignAssets;
    type DripAssetId = DripAssetId;
    type DripAmount = DripAmount;
    type MinXcavBalance = MinXcavBalance;
    type CooldownPeriod = CooldownPeriod;
    type BlockNumberProvider = System;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut test = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            ([1; 32].into(), 10_000_000_000_000_000), // 10_000 XCAV
            ([2; 32].into(), 500_000_000_000),        // 0.5 XCAV (below minimum)
            ([3; 32].into(), 1_000_000_000_000),      // exactly 1 XCAV
        ],
        dev_accounts: None,
    }
    .assimilate_storage(&mut test)
    .unwrap();

    pallet_assets::GenesisConfig::<Test, Instance2> {
        assets: vec![(10, [0; 32].into(), true, 1)],
        metadata: vec![(10, "tGBP".into(), "tGBP".into(), 18)],
        accounts: vec![],
        next_asset_id: None,
    }
    .assimilate_storage(&mut test)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(test);
    ext.execute_with(|| {
        System::set_block_number(1);
    });
    ext
}
