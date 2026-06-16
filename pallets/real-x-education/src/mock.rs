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

use frame_support::{
    derive_impl, parameter_types,
    traits::{AsEnsureOriginWithArg, EnsureOriginWithArg, OriginTrait},
    PalletId,
};
use sp_runtime::{
    traits::{AccountIdLookup, BlakeTwo256, ConstU128, ConstU32, ConstU8, IdentifyAccount, Verify},
    BuildStorage, MultiSignature, Percent,
};

use pallet_nfts::PalletFeatures;

use pallet_assets::{Instance2, Instance3};

use frame_system::EnsureRoot;

pub type Block = frame_system::mocking::MockBlock<Test>;

pub type BlockNumber = u64;

pub type Balance = u128;

pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Signature = MultiSignature;
pub type AccountPublic = <Signature as Verify>::Signer;

pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

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
    pub type EducationAssets = pallet_assets::Pallet<Runtime, Instance3>;
    #[runtime::pallet_index(3)]
    pub type ForeignAssets = pallet_assets::Pallet<Runtime, Instance2>;
    #[runtime::pallet_index(4)]
    pub type EducationNfts = pallet_nfts;
    #[runtime::pallet_index(5)]
    pub type AssetsHolder = pallet_assets_holder::Pallet<Runtime, Instance2>;
    #[runtime::pallet_index(6)]
    pub type XcavateWhitelist = pallet_xcavate_whitelist;
    #[runtime::pallet_index(7)]
    pub type EducationRegions = pallet_education_regions;
    #[runtime::pallet_index(8)]
    pub type RealXEducation = crate;
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

impl pallet_assets::Config<Instance2> for Test {
    type RuntimeEvent = RuntimeEvent;
    type Balance = u128;
    type AssetId = u32;
    type AssetIdParameter = parity_scale_codec::Compact<u32>;
    type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<Self::AccountId>>;
    type ForceOrigin = EnsureRoot<AccountId>;
    type AssetDeposit = ConstU128<1>;
    type AssetAccountDeposit = ConstU128<1>;
    type MetadataDepositBase = ConstU128<1>;
    type MetadataDepositPerByte = ConstU128<1>;
    type ApprovalDeposit = ConstU128<1>;
    type StringLimit = ConstU32<50>;
    type Freezer = ();
    type Holder = AssetsHolder;
    type Extra = ();
    type CallbackHandle = ();
    type WeightInfo = ();
    type RemoveItemsLimit = ConstU32<1000>;
}

impl pallet_assets::Config<Instance3> for Test {
    type RuntimeEvent = RuntimeEvent;
    type Balance = u128;
    type AssetId = u32;
    type AssetIdParameter = parity_scale_codec::Compact<u32>;
    type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<Self::AccountId>>;
    type ForceOrigin = EnsureRoot<AccountId>;
    type AssetDeposit = ConstU128<1>;
    type AssetAccountDeposit = ConstU128<1>;
    type MetadataDepositBase = ConstU128<1>;
    type MetadataDepositPerByte = ConstU128<1>;
    type ApprovalDeposit = ConstU128<1>;
    type StringLimit = ConstU32<50>;
    type Freezer = ();
    type Holder = ();
    type Extra = ();
    type CallbackHandle = ();
    type WeightInfo = ();
    type RemoveItemsLimit = ConstU32<1000>;
}

impl pallet_assets_holder::Config<pallet_assets::Instance2> for Test {
    type RuntimeHoldReason = MarketplaceHoldReason;
    type RuntimeEvent = RuntimeEvent;
}

parameter_types! {
    pub Features: PalletFeatures = PalletFeatures::all_enabled();
    pub const ApprovalsLimit: u32 = 20;
    pub const ItemAttributesApprovalsLimit: u32 = 20;
    pub const MaxTips: u32 = 10;
    pub const MaxDeadlineDuration: BlockNumber = 12 * 30 * DAYS;
    pub const MaxAttributesPerCall: u32 = 10;
}

impl pallet_nfts::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type CollectionId = u32;
    type ItemId = u32;
    type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<Self::AccountId>>;
    type ForceOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type Locker = ();
    type CollectionDeposit = ConstU128<0>;
    type ItemDeposit = ConstU128<0>;
    type MetadataDepositBase = ConstU128<0>;
    type AttributeDepositBase = ConstU128<0>;
    type DepositPerByte = ConstU128<0>;
    type StringLimit = ConstU32<50>;
    type KeyLimit = ConstU32<50>;
    type ValueLimit = ConstU32<50>;
    type WeightInfo = ();
    type ApprovalsLimit = ApprovalsLimit;
    type ItemAttributesApprovalsLimit = ItemAttributesApprovalsLimit;
    type MaxTips = MaxTips;
    type MaxDeadlineDuration = MaxDeadlineDuration;
    type MaxAttributesPerCall = MaxAttributesPerCall;
    type Features = Features;
    type OffchainSignature = Signature;
    type OffchainPublic = AccountPublic;
    type BlockNumberProvider = System;
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
    pub const LocationDepositAmount: Balance = 10_000;
    pub const RegionVotingTime: BlockNumber = 30;
    pub const RegionAuctionTime: BlockNumber = 30;
    pub const RegionThreshold: Percent = Percent::from_percent(75);
    pub const RegionOperatorVotingTime: BlockNumber = 30;
    pub const RegionOwnerChangeTime: BlockNumber = 300;
    pub const RegionOwnerNoticeTime: BlockNumber = 100;
    pub const RegionVotingQuorum: Permill = Permill::from_percent(1);
    pub NewAssetSymbol: BoundedVec<u8, ConstU32<50>> = (*b"FRAC").to_vec().try_into().unwrap();
    pub NewAssetName: BoundedVec<u8, ConstU32<50>> = (*b"Frac").to_vec().try_into().unwrap();
}

impl pallet_education_regions::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_education_regions::weights::SubstrateWeight<Test>;
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

parameter_types! {
    pub const XEducationPalletId: PalletId = PalletId(*b"py/xeduc");
    pub const MaximumModuleToken: u32 = 1000;
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub const ContentCreatorPercentage: Perbill = Perbill::from_parts(83_000_000);
    pub const RegionalOperatorPercentage: Perbill = Perbill::from_parts(83_000_000);
    pub const ProtocolPercentage: Perbill = Perbill::from_parts(50_000_000);
    pub const DBSPercentage: Perbill = Perbill::from_parts(34_000_000);
    pub const AcceptedPaymentAssets: [u32; 3] = [10, 1337, 1984];
    pub const CancellationWindow: BlockNumber = 100;
    pub const MaximumCancellations: u32 = 3;
    pub const SponsorshipWindow: BlockNumber = 200;
    pub const MaximumAllowedStrikes: u8 = 3;
    pub const StrikeSlashPercentage: Perbill = Perbill::from_parts(100_000_000);
    pub const MaxCleanupPerCall: u32 = 50;
    pub const MinimumImpactScore: Permill = Permill::from_percent(50);
    pub const SuccessfulDeliveriesForStrikeReduction: u32 = 5;
}

impl crate::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::SubstrateWeight<Test>;
    type Balance = Balance;
    type NativeCurrency = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type Nfts = EducationNfts;
    type NftCollectionId = <Self as pallet_nfts::Config>::CollectionId;
    type NftId = <Self as pallet_nfts::Config>::ItemId;
    type MaxModuleToken = MaximumModuleToken;
    type LocalCurrency = EducationAssets;
    type ForeignCurrency = ForeignAssets;
    type ForeignAssetsHolder = AssetsHolder;
    type StringLimit = ConstU32<50>;
    type ModulePrice = ConstU128<1000>;
    type BlockNumberProvider = System;
    type ContentCreatorPercentage = ContentCreatorPercentage;
    type RegionalOperatorPercentage = RegionalOperatorPercentage;
    type ProtocolPercentage = ProtocolPercentage;
    type DBSPercentage = DBSPercentage;
    type PalletId = XEducationPalletId;
    type TreasuryId = TreasuryPalletId;
    type PermissionOrigin = EnsureHasRole<Self>;
    type AcceptedAssets = AcceptedPaymentAssets;
    type BookingDeposit = ConstU128<10>;
    type ModuleDeposit = ConstU128<100>;
    type RegionProvider = EducationRegions;
    type NewAssetSymbol = NewAssetSymbol;
    type NewAssetName = NewAssetName;
    type Slash = ();
    type MaxCancellations = MaximumCancellations;
    type CancellationWindow = CancellationWindow;
    type SponsorshipWindow = SponsorshipWindow;
    type ModuleDelivererDeposit = ConstU128<100>;
    type MaxAllowedStrikes = MaximumAllowedStrikes;
    type StrikeSlashPercentage = StrikeSlashPercentage;
    type MaxCleanupPerCall = MaxCleanupPerCall;
    type MinImpactScore = MinimumImpactScore;
    type SuccessfulDeliveriesForStrikeReduction = SuccessfulDeliveriesForStrikeReduction;
    type RoleProvider = XcavateWhitelist;
    type AssetMetadata = AssetsMetadataWrapper;
}

pub struct AssetsMetadataWrapper;

impl AssetMetadataProvider for AssetsMetadataWrapper {
    type AssetId = u32;

    fn get_decimals(asset_id: Self::AssetId) -> Option<u8> {
        Some(pallet_assets::Metadata::<Test, Instance2>::get(asset_id).decimals)
    }
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut test = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            ([0; 32].into(), 20_000),
            ([1; 32].into(), 15_000),
            ([2; 32].into(), 5_000),
            ([3; 32].into(), 5_000),
            ([4; 32].into(), 5_000),
            ([5; 32].into(), 5_000),
            ([8; 32].into(), 250_000),
        ],
        dev_accounts: None,
    }
    .assimilate_storage(&mut test)
    .unwrap();

    pallet_assets::GenesisConfig::<Test, Instance2> {
        assets: vec![(1984, [0; 32].into(), true, 1)], // Genesis assets: id, owner, is_sufficient, min_balance
        metadata: vec![(1984, "USDT".into(), "USDT".into(), 2)], // Genesis metadata: id, name, symbol, decimals
        accounts: vec![(1984, [2; 32].into(), 6_000_000)], // Genesis accounts: id, account_id, balance
        next_asset_id: None,
    }
    .assimilate_storage(&mut test)
    .unwrap();

    pallet_assets::GenesisConfig::<Test, Instance2> {
        assets: vec![(10, [0; 32].into(), true, 1)], // Genesis assets: id, owner, is_sufficient, min_balance
        metadata: vec![(10, "ttGBP".into(), "ttGBP".into(), 6)], // Genesis metadata: id, name, symbol, decimals
        accounts: vec![(10, [2; 32].into(), 60_000_000_000)], // Genesis accounts: id, account_id, balance
        next_asset_id: None,
    }
    .assimilate_storage(&mut test)
    .unwrap();

    test.into()
}
