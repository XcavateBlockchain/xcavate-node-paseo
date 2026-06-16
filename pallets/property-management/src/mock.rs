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

use crate as pallet_property_management;
use frame_support::{
    derive_impl, parameter_types,
    traits::{AsEnsureOriginWithArg, OriginTrait},
    BoundedVec,
};
use sp_core::{ConstU128, ConstU32};
use sp_runtime::{
    traits::{AccountIdLookup, BlakeTwo256, ConstU8, IdentifyAccount, Verify},
    MultiSignature, Perbill, Percent, Permill,
};

use frame_system::EnsureRoot;

use sp_runtime::BuildStorage;

use pallet_nfts::PalletFeatures;

use pallet_assets::{Instance1, Instance2};

use primitives::{MarketplaceFreezeReason, MarketplaceHoldReason};

pub type Block = frame_system::mocking::MockBlock<Test>;

pub type BlockNumber = u64;

pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Signature = MultiSignature;
pub type AccountPublic = <Signature as Verify>::Signer;

pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
        Nfts: pallet_nfts::{Pallet, Call, Storage, Event<T>},
        PropertyManagement: pallet_property_management,
        NftFractionalization: pallet_nft_fractionalization,
        Marketplace: pallet_marketplace,
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        LocalAssets: pallet_assets::<Instance1>,
        ForeignAssets: pallet_assets::<Instance2>,
        XcavateWhitelist: pallet_xcavate_whitelist,
        AssetsHolder: pallet_assets_holder::<Instance2>,
        AssetsFreezer: pallet_assets_freezer::<Instance1>,
        Regions: pallet_regions,
        RealWorldAsset: pallet_real_world_asset,
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
    type MaxConsumers = frame_support::traits::ConstU32<1024>;
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
    type CollectionDeposit = ConstU128<2>;
    type ItemDeposit = ConstU128<1>;
    type MetadataDepositBase = ConstU128<1>;
    type AttributeDepositBase = ConstU128<1>;
    type DepositPerByte = ConstU128<1>;
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
    pub const AssetConversionPalletId: PalletId = PalletId(*b"py/ascon");

}

impl pallet_assets::Config<Instance1> for Test {
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
    type Freezer = AssetsFreezer;
    type Holder = ();
    type Extra = ();
    type CallbackHandle = ();
    type WeightInfo = ();
    type RemoveItemsLimit = ConstU32<1000>;
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

impl pallet_assets_holder::Config<pallet_assets::Instance2> for Test {
    type RuntimeHoldReason = MarketplaceHoldReason;
    type RuntimeEvent = RuntimeEvent;
}

impl pallet_assets_freezer::Config<pallet_assets::Instance1> for Test {
    type RuntimeFreezeReason = MarketplaceFreezeReason;
    type RuntimeEvent = RuntimeEvent;
}

parameter_types! {
    pub const NftFractionalizationPalletId: PalletId = PalletId(*b"fraction");
    pub NewAssetSymbol: BoundedVec<u8, ConstU32<50>> = (*b"FRAC").to_vec().try_into().unwrap();
    pub NewAssetName: BoundedVec<u8, ConstU32<50>> = (*b"Frac").to_vec().try_into().unwrap();
}

impl pallet_nft_fractionalization::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Deposit = ConstU128<1>;
    type Currency = Balances;
    type NewAssetSymbol = NewAssetSymbol;
    type NewAssetName = NewAssetName;
    type NftCollectionId = <Self as pallet_nfts::Config>::CollectionId;
    type NftId = <Self as pallet_nfts::Config>::ItemId;
    type AssetBalance = <Self as pallet_balances::Config>::Balance;
    type AssetId = <Self as pallet_assets::Config<Instance1>>::AssetId;
    type Assets = LocalAssets;
    type Nfts = Nfts;
    type PalletId = NftFractionalizationPalletId;
    type WeightInfo = ();
    type StringLimit = ConstU32<50>;
    type RuntimeHoldReason = RuntimeHoldReason;
}

parameter_types! {
    pub const AirdropNativeAmount: u128 = 0;
    pub const AirdropAssetId: u32 = 10;
    pub const AirdropAssetAmount: u128 = 0;
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

pub struct EnsureCompliant<T>(core::marker::PhantomData<T>);

impl<T: whitelist::Config> EnsureOriginWithArg<T::RuntimeOrigin, whitelist::Role>
    for EnsureCompliant<T>
{
    type Success = T::AccountId;

    fn try_origin(
        origin: T::RuntimeOrigin,
        role: &whitelist::Role,
    ) -> Result<Self::Success, T::RuntimeOrigin> {
        let Some(who) = origin.clone().into_signer() else {
            return Err(origin);
        };
        if whitelist::Pallet::<T>::is_compliant(&who, role.clone()) {
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
    pub const MaximumListingDuration: u64 = 10_000;
    pub const RegionVotingTime: BlockNumber = 30;
    pub const RegionAuctionTime: BlockNumber = 30;
    pub const RegionThreshold: Percent = Percent::from_percent(75);
    pub const RegionOperatorVotingTime: BlockNumber = 30;
    pub const RegionOwnerChangeTime: BlockNumber = 300;
    pub const RegionOwnerNoticeTime: BlockNumber = 100;
    pub const MaximumTaxPercent: Permill = Permill::from_percent(10);
    pub const RegionVotingQuorum: Permill = Permill::from_percent(1);
}

impl pallet_regions::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_regions::weights::SubstrateWeight<Test>;
    type Balance = u128;
    type NativeCurrency = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type Nfts = Nfts;
    type NftCollectionId = <Self as pallet_nfts::Config>::CollectionId;
    type NftId = <Self as pallet_nfts::Config>::ItemId;
    type MarketplacePalletId = MarketplacePalletId;
    type MaxListingDuration = MaximumListingDuration;
    type PostcodeLimit = Postcode;
    type LocationDeposit = ConstU128<10_000>;
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
    type LawyerDeposit = ConstU128<10_000>;
    type BlockNumberProvider = System;
    type MaxTaxPercent = MaximumTaxPercent;
    type AllowedStrikes = ConstU8<3>;
    type MinVotingQuorum = RegionVotingQuorum;
}

impl pallet_real_world_asset::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Balance = u128;
    type NativeCurrency = Balances;
    type NftCollectionId = <Self as pallet_nfts::Config>::CollectionId;
    type NftId = <Self as pallet_nfts::Config>::ItemId;
    type Nfts = Nfts;
    type MarketplacePalletId = MarketplacePalletId;
    type LocalCurrency = LocalAssets;
    type FractionalizeCollectionId = <Self as pallet_nfts::Config>::CollectionId;
    type FractionalizeItemId = <Self as pallet_nfts::Config>::ItemId;
    type AssetId = <Self as pallet_assets::Config<Instance1>>::AssetId;
    type PropertyAccountFundingAmount = ConstU128<100>;
    type MaxPropertyShares = MaxPropertyShares;
    type StringLimit = ConstU32<50>;
    type RegionProvider = Regions;
    type PostcodeLimit = Postcode;
    type NamespaceManager = ();
}

parameter_types! {
    pub const MarketplacePalletId: PalletId = PalletId(*b"py/nftxc");
    pub const MinPropertyShares: u32 = 100;
    pub const MaxPropertyShares: u32 = 1000;
    pub const MaxNftsInCollection: u32 = 100;
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
    pub const AcceptedPaymentAssets: [u32; 2] = [1337, 1984];
    pub const MaximumAcceptedAssets: u32 = 2;
    pub const LawyerVotingDuration: BlockNumber = 30;
    pub const LegalProcessDuration: BlockNumber = 150;
    pub const MinimumVotingQuorum: Percent = Percent::from_percent(50);
    pub const ClaimWindowTime: BlockNumber = 100;
    pub const MaximumRelistAttempts: u8 = 1;
    pub const MarketplaceFeePercent: Perbill = Perbill::from_percent(1);
    pub const MaxOwnershipPercentage: Perbill = Perbill::from_percent(50);
}

/// Configure the pallet-xcavate-staking in pallets/xcavate-staking.
impl pallet_marketplace::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_marketplace::weights::SubstrateWeight<Test>;
    type Balance = u128;
    type NativeCurrency = Balances;
    type RuntimeHoldReason = RuntimeHoldReason;
    type LocalCurrency = LocalAssets;
    type ForeignCurrency = ForeignAssets;
    type ForeignAssetsHolder = AssetsHolder;
    type AssetsFreezer = AssetsFreezer;
    type NftCollectionId = <Self as pallet_nfts::Config>::CollectionId;
    type NftId = <Self as pallet_nfts::Config>::ItemId;
    type PalletId = MarketplacePalletId;
    type MinPropertyShares = MinPropertyShares;
    type MaxPropertyShares = MaxPropertyShares;
    type TreasuryId = TreasuryPalletId;
    type AssetId = <Self as pallet_assets::Config<Instance1>>::AssetId;
    type ListingDeposit = ConstU128<10>;
    type MarketplaceFeePercentage = MarketplaceFeePercent;
    type AcceptedAssets = AcceptedPaymentAssets;
    type MaxAcceptedAssets = MaximumAcceptedAssets;
    type PropertyShares = RealWorldAsset;
    type LawyerVotingTime = LawyerVotingDuration;
    type LegalProcessTime = LegalProcessDuration;
    type Whitelist = XcavateWhitelist;
    type PermissionOrigin = EnsureHasRole<Self>;
    type CompliantOrigin = EnsureCompliant<Self>;
    type MinVotingQuorum = MinimumVotingQuorum;
    type ClaimWindow = ClaimWindowTime;
    type MaxRelistAttempts = MaximumRelistAttempts;
    type BlockNumberProvider = System;
    type IncomeSettlement = PropertyManagement;
    type RegionProvider = Regions;
    type StringLimit = ConstU32<50>;
    type PostcodeLimit = Postcode;
    type MaxOwnershipPercentage = MaxOwnershipPercentage;
}

parameter_types! {
    pub const MaxProperty: u32 = 100;
    pub const MaxLocation: u32 = 100;
    pub const LettingAgentVotingDuration: BlockNumber = 30;
    pub const LettingAgentNoticeTime: BlockNumber = 30;
    pub const MaximumNoticesPerBlock: u32 = 10;
}

/// Configure the pallet-property-management in pallets/property-management.
impl pallet_property_management::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = weights::SubstrateWeight<Test>;
    type Balance = u128;
    type RuntimeHoldReason = RuntimeHoldReason;
    type NativeCurrency = Balances;
    type ForeignCurrency = ForeignAssets;
    type AssetsFreezer = AssetsFreezer;
    type NftCollectionId = <Self as pallet_nfts::Config>::CollectionId;
    type NftId = <Self as pallet_nfts::Config>::ItemId;
    type MarketplacePalletId = MarketplacePalletId;
    type LettingAgentDeposit = ConstU128<1000>;
    type MaxProperties = MaxProperty;
    type MaxLocations = MaxLocation;
    type AcceptedAssets = AcceptedPaymentAssets;
    type PropertyShares = RealWorldAsset;
    type LettingAgentVotingTime = LettingAgentVotingDuration;
    type PermissionOrigin = EnsureHasRole<Self>;
    type MinVotingQuorum = MinimumVotingQuorum;
    type LettingAgentNoticePeriod = LettingAgentNoticeTime;
    type MaxNoticesPerBlock = MaximumNoticesPerBlock;
    type BlockNumberProvider = System;
    type RegionProvider = Regions;
    type PostcodeLimit = Postcode;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut test = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            ([0; 32].into(), 20_000_000),
            ([1; 32].into(), 15_000_000),
            ([2; 32].into(), 1_150_000),
            ([3; 32].into(), 1_005_000),
            ([4; 32].into(), 5_000),
            ([6; 32].into(), 2_000_000),
            ([10; 32].into(), 15_000),
            ([11; 32].into(), 15_000),
            ([15; 32].into(), 5_000),
            ([16; 32].into(), 5_000),
            ([30; 32].into(), 5_000),
            ([31; 32].into(), 5_000),
            ((Marketplace::account_id()), 20_000_000),
            ((PropertyManagement::property_account_id(0)), 5_000),
        ],
        dev_accounts: None,
    }
    .assimilate_storage(&mut test)
    .unwrap();

    pallet_assets::GenesisConfig::<Test, Instance2> {
        assets: vec![(1984, [0; 32].into(), true, 1)], // Genesis assets: id, owner, is_sufficient, min_balance
        metadata: vec![(1984, "USDT".into(), "USDT".into(), 0)], // Genesis metadata: id, name, symbol, decimals
        accounts: vec![
            (1984, [0; 32].into(), 20_000_000),
            (1984, [1; 32].into(), 1_500_000),
            (1984, [2; 32].into(), 1_150_000),
            (1984, [3; 32].into(), 1_150_000),
            (1984, [4; 32].into(), 5_000),
            (1984, [5; 32].into(), 500),
            (1984, [15; 32].into(), 20_000_000),
            (1984, [16; 32].into(), 20_000_000),
            (1984, [30; 32].into(), 1_200_000),
            (1984, [31; 32].into(), 1_200_000),
        ], // Genesis accounts: id, account_id, balance
        next_asset_id: None,
    }
    .assimilate_storage(&mut test)
    .unwrap();

    pallet_assets::GenesisConfig::<Test, Instance2> {
        assets: vec![(1337, [0; 32].into(), true, 1)], // Genesis assets: id, owner, is_sufficient, min_balance
        metadata: vec![(1337, "USDT".into(), "USDT".into(), 0)], // Genesis metadata: id, name, symbol, decimals
        accounts: vec![(1337, [4; 32].into(), 5_000)], // Genesis accounts: id, account_id, balance
        next_asset_id: None,
    }
    .assimilate_storage(&mut test)
    .unwrap();

    test.into()
}
