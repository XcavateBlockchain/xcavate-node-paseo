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

use super::*;

use frame_support::{derive_impl, parameter_types};
use sp_runtime::{
    traits::{AccountIdLookup, BlakeTwo256, ConstU128, ConstU32, ConstU8, IdentifyAccount, Verify},
    BuildStorage, MultiSignature,
};

use pallet_assets::Instance2;

pub type Block = frame_system::mocking::MockBlock<Test>;

pub type BlockNumber = u64;

pub type Balance = u128;

pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Signature = MultiSignature;

// Configure a mock runtime to test the pallet.
#[frame_support::runtime]
mod test_runtime {
    #[runtime::runtime]
    #[runtime::derive(
        RuntimeCall,
        RuntimeEvent,
        RuntimeError,
        RuntimeOrigin,
        RuntimeFreezeReason,
        RuntimeHoldReason,
        RuntimeSlashReason,
        RuntimeLockId,
        RuntimeTask
    )]
    pub struct Test;

    #[runtime::pallet_index(0)]
    pub type System = frame_system;
    #[runtime::pallet_index(1)]
    pub type Balances = pallet_balances;
    #[runtime::pallet_index(2)]
    pub type XcavateWhitelist = pallet_xcavate_whitelist;
    #[runtime::pallet_index(3)]
    pub type Regions = crate;
    #[runtime::pallet_index(4)]
    pub type ForeignAssets = pallet_assets::Pallet<Runtime, Instance2>;
}

parameter_types! {
    pub const BlockHashCount: BlockNumber = 2400;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
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
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
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
    pub const AirdropNativeAmount: Balance = 0;
    pub const AirdropAssetId: u32 = 10;
    pub const AirdropAssetAmount: Balance = 0;
}

impl pallet_xcavate_whitelist::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_xcavate_whitelist::weights::SubstrateWeight<Test>;
    type WhitelistOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type Balance = u128;
    type NativeCurrency = Balances;
    type ForeignCurrency = ForeignAssets;
    type AirdropNativeAmount = AirdropNativeAmount;
    type AirdropAssetId = AirdropAssetId;
    type AirdropAssetAmount = AirdropAssetAmount;
}

use pallet_xcavate_whitelist::{self as whitelist, RolePermission};

pub struct EnsureHasRole<T>(core::marker::PhantomData<T>);

impl<T: whitelist::Config> EnsureOriginWithArg<T::RuntimeOrigin, whitelist::Role>
    for EnsureHasRole<T>
{
    type Success = T::AccountId;

    fn try_origin(
        origin: T::RuntimeOrigin,
        role: &whitelist::Role,
    ) -> Result<Self::Success, T::RuntimeOrigin> {
        let Some(who) = origin.clone().into_signer() else {
            return Err(origin);
        };
        if whitelist::Pallet::<T>::has_role(&who, role.clone()) {
            Ok(who)
        } else {
            Err(origin)
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn try_successful_origin(_role: &whitelist::Role) -> Result<T::RuntimeOrigin, ()> {
        let account = frame_benchmarking::whitelisted_caller();
        Ok(frame_system::RawOrigin::Signed(account).into())
    }
}

parameter_types! {
    pub const Postcode: u32 = 10;
    pub const MaximumListingDuration: BlockNumber = 10_000;
    pub const RegionVotingTime: BlockNumber = 30;
    pub const RegionAuctionTime: BlockNumber = 30;
    pub const RegionThreshold: Percent = Percent::from_percent(75);
    pub const RegionOperatorVotingTime: BlockNumber = 30;
    pub const RegionOwnerChangeTime: BlockNumber = 300;
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub const RegionOwnerNoticeTime: BlockNumber = 100;
    pub const MaximumTaxPercent: Permill = Permill::from_percent(10);
    pub const RegionVotingQuorum: Permill = Permill::from_percent(1);
}

impl crate::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::SubstrateWeight<Test>;
    type Balance = u128;
    type NativeCurrency = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RegionVotingTime = RegionVotingTime;
    type RegionAuctionTime = RegionAuctionTime;
    type RegionThreshold = RegionThreshold;
    type RegionOperatorVotingTime = RegionOperatorVotingTime;
    type MaxProposalsForBlock = ConstU32<100>;
    type RegionSlashingAmount = ConstU128<10_000>;
    type TreasuryId = TreasuryPalletId;
    type RegionOwnerChangePeriod = RegionOwnerChangeTime;
    type Slash = ();
    type RegionOwnerNoticePeriod = RegionOwnerNoticeTime;
    type RegionOwnerDisputeDeposit = ConstU128<1_000>;
    type MinimumRegionDeposit = ConstU128<10_000>;
    type RegionProposalDeposit = ConstU128<5_000>;
    type MinimumVotingAmount = ConstU128<100>;
    type PermissionOrigin = EnsureHasRole<Self>;
    type BlockNumberProvider = System;
    type AllowedStrikes = ConstU8<3>;
    type MinVotingQuorum = RegionVotingQuorum;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut test = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            ([0; 32].into(), 200_000),
            ([1; 32].into(), 150_000),
            ([2; 32].into(), 300_000),
            ([3; 32].into(), 5_000),
            ([8; 32].into(), 400_000),
        ],
        dev_accounts: None,
    }
    .assimilate_storage(&mut test)
    .unwrap();

    test.into()
}
