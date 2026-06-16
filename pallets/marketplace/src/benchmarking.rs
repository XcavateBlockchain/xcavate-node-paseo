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

//! Benchmarking setup for pallet-marketplace
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as Marketplace;
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_support::traits::Get;
use frame_support::BoundedVec;
use frame_system::RawOrigin;
use pallet_regions::Pallet as Regions;
use pallet_regions::{RegionIdentifier, Vote};
use pallet_xcavate_whitelist::Pallet as Whitelist;
use pallet_xcavate_whitelist::Role;
use scale_info::prelude::vec;
extern crate alloc;
use alloc::vec::Vec;

pub trait Config:
    pallet_xcavate_whitelist::Config + pallet_regions::Config + crate::Config
{
}

impl<T: crate::Config + pallet_xcavate_whitelist::Config + pallet_regions::Config> Config for T {}

fn create_whitelisted_user<T: Config>() -> (T::AccountId, T::AccountId) {
    let admin: T::AccountId = account("admin", 0, 0);
    let signer: T::AccountId = account("signer", 0, 0);
    assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));
    assert_ok!(Whitelist::<T>::assign_role(
        RawOrigin::Signed(admin.clone()).into(),
        signer.clone(),
        Role::RealEstateDeveloper
    ));
    (signer, admin)
}

fn create_a_new_region<T: Config>(
    signer: T::AccountId,
    admin: T::AccountId,
) -> (u16, LocationId<T>) {
    let region = RegionIdentifier::France;
    let region_id = region.clone().into_u16();

    let deposit = T::RegionProposalDeposit::get();
    let auction_amount = T::MinimumRegionDeposit::get();
    let total_funds = deposit
        .saturating_mul(1000u32.into())
        .saturating_add(auction_amount.saturating_mul(100u32.into()));
    assert_ok!(<T as pallet_regions::Config>::NativeCurrency::mint_into(&signer, total_funds));
    assert_ok!(Whitelist::<T>::assign_role(
        RawOrigin::Signed(admin.clone()).into(),
        signer.clone(),
        Role::RegionalOperator
    ));
    assert_ok!(Whitelist::<T>::assign_role(
        RawOrigin::Signed(admin.clone()).into(),
        signer.clone(),
        Role::RealEstateInvestor
    ));
    assert_ok!(Regions::<T>::propose_new_region(
        RawOrigin::Signed(signer.clone()).into(),
        region.clone()
    ));
    assert_ok!(Regions::<T>::vote_on_region_proposal(
        RawOrigin::Signed(signer.clone()).into(),
        region_id,
        Vote::Yes,
        deposit.saturating_mul(800u32.into())
    ));

    let bid_amount = auction_amount.saturating_mul(10u32.into());

    let expiry = frame_system::Pallet::<T>::block_number() + T::RegionVotingTime::get();
    frame_system::Pallet::<T>::set_block_number(expiry);

    assert_ok!(Regions::<T>::bid_on_region(
        RawOrigin::Signed(signer.clone()).into(),
        region_id,
        bid_amount
    ));

    let auction_expiry = frame_system::Pallet::<T>::block_number() + T::RegionAuctionTime::get();
    frame_system::Pallet::<T>::set_block_number(auction_expiry);
    assert_ok!(Regions::<T>::create_new_region(
        RawOrigin::Signed(signer.clone()).into(),
        region_id,
        T::MaxListingDuration::get(),
        Permill::from_percent(5)
    ));

    let location = BoundedVec::try_from("SG23 5TH".as_bytes().to_vec()).unwrap();
    assert_ok!(Regions::<T>::create_new_location(
        RawOrigin::Signed(signer.clone()).into(),
        region_id,
        location.clone()
    ));

    // Verify region and location
    assert!(pallet_regions::RegionDetails::<T>::contains_key(region_id));
    assert!(pallet_regions::LocationRegistration::<T>::contains_key(region_id, &location));
    let location = BoundedVec::try_from("SG23 5TH".as_bytes().to_vec()).unwrap();

    (region_id, location)
}

// Helper function to list a property
fn fund_pallet_account<T: Config>() {
    let pallet_account = Marketplace::<T>::account_id();
    let _ = <T as pallet::Config>::NativeCurrency::mint_into(
        &pallet_account,
        1_000_000_000_000_000u128.into(),
    );
}

fn list_property_helper<T: Config>(
    seller: T::AccountId,
    region_id: u16,
    location: LocationId<T>,
    share_amount: u32,
    share_price: <T as pallet::Config>::Balance,
    tax_paid_by_developer: bool,
) -> u32 {
    fund_pallet_account::<T>();
    let property_price = share_price.saturating_mul((share_amount as u128).into());
    let deposit_amount = property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
    assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
        &seller,
        deposit_amount.saturating_mul(20u32.into())
    ));

    let metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
        BoundedVec::truncate_from(vec![42u8; <T as pallet::Config>::StringLimit::get() as usize]);

    assert_ok!(Marketplace::<T>::list_property(
        RawOrigin::Signed(seller).into(),
        region_id,
        location,
        share_price,
        share_amount,
        metadata,
        tax_paid_by_developer,
    ));
    let listing_id = 0;
    assert!(OngoingObjectListing::<T>::contains_key(listing_id));
    listing_id
}

fn list_and_sell_property<T: Config>(
    seller: T::AccountId,
    region_id: u16,
    location: LocationId<T>,
    admin: T::AccountId,
) -> T::AccountId {
    fund_pallet_account::<T>();
    let share_amount: u32 = <T as pallet::Config>::MaxPropertyShares::get();
    let share_price: <T as pallet::Config>::Balance = 1_000u32.into();
    let property_price = share_price.saturating_mul((share_amount as u128).into());
    let deposit_amount = property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
    assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
        &seller,
        deposit_amount.saturating_mul(20u32.into())
    ));

    let metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
        BoundedVec::truncate_from(vec![42u8; <T as pallet::Config>::StringLimit::get() as usize]);

    let tax_paid_by_developer = true;
    assert_ok!(Marketplace::<T>::list_property(
        RawOrigin::Signed(seller).into(),
        region_id,
        location,
        share_price,
        share_amount,
        metadata,
        tax_paid_by_developer,
    ));
    let listing_id = 0;
    assert!(OngoingObjectListing::<T>::contains_key(listing_id));
    let payment_asset = T::AcceptedAssets::get()[0];
    let buyer: T::AccountId = account("buyer", 0, 0);
    assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
        &buyer,
        deposit_amount.saturating_mul(20u32.into())
    ));
    assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
        payment_asset,
        &buyer,
        property_price.saturating_mul(100u32.into())
    ));
    assert_ok!(Whitelist::<T>::assign_role(
        RawOrigin::Signed(admin.clone()).into(),
        buyer.clone(),
        Role::RealEstateInvestor
    ));
    add_buyers_to_listing::<T>(share_amount - 1, payment_asset, property_price, admin.clone());

    assert_ok!(Marketplace::<T>::buy_property_shares(
        RawOrigin::Signed(buyer.clone()).into(),
        listing_id,
        1,
        payment_asset,
    ));
    let spv_admin: T::AccountId = account("spv_admin", 0, 0);
    assert_ok!(Whitelist::<T>::assign_role(
        RawOrigin::Signed(admin).into(),
        spv_admin.clone(),
        Role::SpvConfirmation
    ));
    assert_ok!(Marketplace::<T>::create_spv(RawOrigin::Signed(spv_admin).into(), listing_id,));
    assert_ok!(Marketplace::<T>::claim_property_shares(
        RawOrigin::Signed(buyer.clone()).into(),
        listing_id,
    ));
    claim_buyers_property_shares::<T>(share_amount - 1, listing_id);
    buyer
}

fn create_registered_property<T: Config>(
    seller: T::AccountId,
    region_id: u16,
    location: LocationId<T>,
    admin: T::AccountId,
) -> T::AccountId {
    let share_owner =
        list_and_sell_property::<T>(seller.clone(), region_id, location, admin.clone());
    let lawyer_1: T::AccountId = account("lawyer1", 0, 0);
    let lawyer_2: T::AccountId = account("lawyer2", 0, 0);
    assert_ok!(Whitelist::<T>::assign_role(
        RawOrigin::Signed(admin.clone()).into(),
        lawyer_1.clone(),
        Role::Lawyer
    ));
    let lawyer_deposit = <T as pallet_regions::Config>::LawyerDeposit::get();
    let _ = <T as pallet_regions::Config>::NativeCurrency::mint_into(
        &lawyer_1,
        lawyer_deposit * 10u32.into(),
    );
    assert_ok!(Whitelist::<T>::assign_role(
        RawOrigin::Signed(admin).into(),
        lawyer_2.clone(),
        Role::Lawyer
    ));
    assert_ok!(Regions::<T>::register_lawyer(
        RawOrigin::Signed(lawyer_1.clone()).into(),
        region_id,
    ));
    let lawyer_deposit = <T as pallet_regions::Config>::LawyerDeposit::get();
    let _ = <T as pallet_regions::Config>::NativeCurrency::mint_into(
        &lawyer_2,
        lawyer_deposit * 10u32.into(),
    );
    assert_ok!(Regions::<T>::register_lawyer(
        RawOrigin::Signed(lawyer_2.clone()).into(),
        region_id,
    ));
    assert_ok!(Marketplace::<T>::lawyer_claim_property(
        RawOrigin::Signed(lawyer_1.clone()).into(),
        0,
        crate::LegalProperty::RealEstateDeveloperSide,
        400_u32.into()
    ));
    assert_ok!(Marketplace::<T>::approve_developer_lawyer(
        RawOrigin::Signed(seller.clone()).into(),
        0,
        true
    ));
    assert_ok!(Marketplace::<T>::lawyer_claim_property(
        RawOrigin::Signed(lawyer_2.clone()).into(),
        0,
        crate::LegalProperty::SpvSide,
        400_u32.into()
    ));
    for i in 1..=<T as pallet::Config>::MaxPropertyShares::get() - 1 {
        let buyer: T::AccountId = account("buyer", i, i);
        assert_ok!(Marketplace::<T>::vote_on_spv_lawyer(
            RawOrigin::Signed(buyer).into(),
            0,
            types::Vote::Yes,
            1
        ));
    }
    let expiry = frame_system::Pallet::<T>::block_number() + T::LawyerVotingTime::get();
    frame_system::Pallet::<T>::set_block_number(expiry);
    assert_ok!(Marketplace::<T>::finalize_spv_lawyer(
        RawOrigin::Signed(share_owner.clone()).into(),
        0,
    ));

    assert_ok!(Marketplace::<T>::lawyer_confirm_documents(
        RawOrigin::Signed(lawyer_1).into(),
        0,
        true
    ));
    assert_ok!(Marketplace::<T>::lawyer_confirm_documents(
        RawOrigin::Signed(lawyer_2).into(),
        0,
        true
    ));
    share_owner
}

fn add_buyers_to_listing<T: Config>(
    buyers: u32,
    payment_asset: u32,
    property_price: <T as pallet::Config>::Balance,
    admin: T::AccountId,
) {
    let deposit_amount = property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();

    for i in 1..=buyers {
        let buyer: T::AccountId = account("buyer", i, i);
        let payment_asset_buyers = T::AcceptedAssets::get()[0];
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            buyer.clone(),
            Role::RealEstateInvestor
        ));
        assert_ok!(Marketplace::<T>::buy_property_shares(
            RawOrigin::Signed(buyer).into(),
            0,
            1,
            payment_asset_buyers
        ));
    }
}

fn claim_buyers_property_shares<T: Config>(buyers: u32, listing_id: ListingId) {
    for i in 1..=buyers {
        let buyer: T::AccountId = account("buyer", i, i);
        assert_ok!(Marketplace::<T>::claim_property_shares(
            RawOrigin::Signed(buyer).into(),
            listing_id
        ));
    }
}

// Helper function to setup SPV and lawyers
fn setup_spv_and_lawyers<T: Config>(
    seller: T::AccountId,
    admin: T::AccountId,
    listing_id: u32,
    region_id: u16,
    buyer: T::AccountId,
    buyers: Vec<T::AccountId>,
) -> (T::AccountId, T::AccountId) {
    let lawyer_1: T::AccountId = account("lawyer1", 0, 0);
    let lawyer_2: T::AccountId = account("lawyer2", 0, 0);
    let lawyer_deposit = <T as pallet_regions::Config>::LawyerDeposit::get();
    assert_ok!(<T as pallet_regions::Config>::NativeCurrency::mint_into(
        &lawyer_1,
        lawyer_deposit * 10u32.into()
    ));
    assert_ok!(Whitelist::<T>::assign_role(
        RawOrigin::Signed(admin.clone()).into(),
        lawyer_1.clone(),
        Role::Lawyer
    ));
    assert_ok!(Regions::<T>::register_lawyer(
        RawOrigin::Signed(lawyer_1.clone()).into(),
        region_id,
    ));
    assert_ok!(<T as pallet_regions::Config>::NativeCurrency::mint_into(
        &lawyer_2,
        lawyer_deposit * 10u32.into()
    ));
    assert_ok!(Whitelist::<T>::assign_role(
        RawOrigin::Signed(admin.clone()).into(),
        lawyer_2.clone(),
        Role::Lawyer
    ));
    assert_ok!(Regions::<T>::register_lawyer(
        RawOrigin::Signed(lawyer_2.clone()).into(),
        region_id,
    ));
    assert_ok!(Marketplace::<T>::lawyer_claim_property(
        RawOrigin::Signed(lawyer_1.clone()).into(),
        listing_id,
        crate::LegalProperty::RealEstateDeveloperSide,
        400_u32.into()
    ));
    assert_ok!(Marketplace::<T>::approve_developer_lawyer(
        RawOrigin::Signed(seller).into(),
        listing_id,
        true
    ));
    assert_ok!(Marketplace::<T>::lawyer_claim_property(
        RawOrigin::Signed(lawyer_2.clone()).into(),
        listing_id,
        crate::LegalProperty::SpvSide,
        400_u32.into()
    ));

    for helper_buyer in buyers {
        let share_amount = T::PropertyShares::get_share_balance(0, &helper_buyer);
        assert_ok!(Marketplace::<T>::vote_on_spv_lawyer(
            RawOrigin::Signed(helper_buyer).into(),
            listing_id,
            types::Vote::Yes,
            share_amount
        ));
    }

    let expiry = frame_system::Pallet::<T>::block_number() + T::LawyerVotingTime::get();
    frame_system::Pallet::<T>::set_block_number(expiry);
    assert_ok!(Marketplace::<T>::finalize_spv_lawyer(RawOrigin::Signed(buyer).into(), listing_id,));

    (lawyer_1, lawyer_2)
}

#[benchmarks]
mod benchmarks {
    use super::*;
    #[benchmark]
    fn list_property(m: Linear<0, { <T as pallet::Config>::StringLimit::get() }>) {
        let (signer, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(signer.clone(), admin);
        let share_amount: u32 = <T as pallet::Config>::MaxPropertyShares::get();
        let share_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let property_price = share_price.saturating_mul((share_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &signer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        fund_pallet_account::<T>();

        let metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![42u8; m as usize]);

        let tax_paid_by_developer = true;
        assert!(!OngoingObjectListing::<T>::contains_key(0));
        #[extrinsic_call]
        list_property(
            RawOrigin::Signed(signer.clone()),
            region_id,
            location,
            share_price,
            share_amount,
            metadata,
            tax_paid_by_developer,
        );

        let listing_id = 0;
        let listing = OngoingObjectListing::<T>::get(listing_id).unwrap();
        assert_eq!(listing.listed_share_amount, share_amount);
        assert_eq!(ListingDeposits::<T>::get(listing_id).unwrap().0, signer);
        assert_eq!(listing.share_price, share_price);
        assert_eq!(listing.tax_paid_by_developer, tax_paid_by_developer);
    }

    #[benchmark]
    fn buy_property_shares_single_share(
        a: Linear<1, { <T as pallet::Config>::MaxPropertyShares::get().saturating_sub(1) }>,
        b: Linear<0, { <T as pallet::Config>::MaxPropertyShares::get().saturating_sub(2) }>,
    ) {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_amount: u32 = <T as pallet::Config>::MaxPropertyShares::get();
        let share_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let listing_id =
            list_property_helper::<T>(seller, region_id, location, share_amount, share_price, true);
        let property_price = share_price.saturating_mul((share_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        let payment_asset = T::AcceptedAssets::get()[0];
        add_buyers_to_listing::<T>(b, payment_asset, property_price, admin.clone());

        let buyer: T::AccountId = account("buyer_final", 0, 0);
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            buyer.clone(),
            Role::RealEstateInvestor
        ));
        let amount: u32 = a.min(share_amount - b - 1);

        let buyer_amount = if amount >= share_amount * 50 / 100 {
            let base = amount / 3;
            let remainder = amount % 3;
            for i in 1..=2 {
                let buyer_helper: T::AccountId = account("buyer_helper", i, i);
                assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
                    &buyer_helper,
                    deposit_amount.saturating_mul(20u32.into())
                ));
                assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
                    payment_asset,
                    &buyer_helper,
                    property_price.saturating_mul(100u32.into())
                ));
                assert_ok!(Whitelist::<T>::assign_role(
                    RawOrigin::Signed(admin.clone()).into(),
                    buyer_helper.clone(),
                    Role::RealEstateInvestor
                ));
                assert_ok!(Marketplace::<T>::buy_property_shares(
                    RawOrigin::Signed(buyer_helper.clone()).into(),
                    listing_id,
                    base,
                    payment_asset,
                ));
            }
            base + remainder
        } else {
            amount
        };

        #[extrinsic_call]
        buy_property_shares(
            RawOrigin::Signed(buyer.clone()),
            listing_id,
            buyer_amount,
            payment_asset,
        );

        assert_eq!(
            OngoingObjectListing::<T>::get(listing_id).unwrap().listed_share_amount,
            share_amount - amount - b
        );
        let share_owner = ShareOwner::<T>::get(&buyer, listing_id).unwrap();
        assert_eq!(share_owner.share_amount, buyer_amount);
        assert!(PropertyLawyer::<T>::get(listing_id).is_none());
    }

    #[benchmark]
    fn buy_property_shares_all_shares(
        b: Linear<1, { <T as pallet::Config>::MaxPropertyShares::get() }>,
        n: Linear<1, { <T as pallet::Config>::AcceptedAssets::get().len() as u32 }>,
    ) {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_amount: u32 = <T as pallet::Config>::MaxPropertyShares::get();
        let share_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let listing_id =
            list_property_helper::<T>(seller, region_id, location, share_amount, share_price, true);
        let property_price = share_price.saturating_mul((share_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        let payment_asset = T::AcceptedAssets::get()[0];
        add_buyers_to_listing::<T>(b - 1, payment_asset, property_price, admin.clone());

        let buyer: T::AccountId = account("buyer_final", 0, 0);
        let amount: u32 = share_amount - (b - 1);
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            buyer.clone(),
            Role::RealEstateInvestor
        ));
        let buyer_amount = if amount >= share_amount * 50 / 100 {
            let base = amount / 3;
            let remainder = amount % 3;
            for i in 1..=2 {
                let buyer_helper: T::AccountId = account("buyer_helper", i, i);
                assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
                    &buyer_helper,
                    deposit_amount.saturating_mul(20u32.into())
                ));
                assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
                    payment_asset,
                    &buyer_helper,
                    property_price.saturating_mul(100u32.into())
                ));
                assert_ok!(Whitelist::<T>::assign_role(
                    RawOrigin::Signed(admin.clone()).into(),
                    buyer_helper.clone(),
                    Role::RealEstateInvestor
                ));
                assert_ok!(Marketplace::<T>::buy_property_shares(
                    RawOrigin::Signed(buyer_helper.clone()).into(),
                    listing_id,
                    base,
                    payment_asset,
                ));
            }
            base + remainder
        } else {
            amount
        };

        #[extrinsic_call]
        buy_property_shares(
            RawOrigin::Signed(buyer.clone()),
            listing_id,
            buyer_amount,
            payment_asset,
        );

        assert_eq!(OngoingObjectListing::<T>::get(listing_id).unwrap().listed_share_amount, 0);
        let share_owner = ShareOwner::<T>::get(&buyer, listing_id).unwrap();
        assert_eq!(share_owner.share_amount, buyer_amount);
        assert_eq!(OngoingObjectListing::<T>::get(listing_id).unwrap().listed_share_amount, 0);
    }

    #[benchmark]
    fn claim_property_shares() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());

        let share_amount: u32 = <T as pallet::Config>::MaxPropertyShares::get();
        let share_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let listing_id =
            list_property_helper::<T>(seller, region_id, location, share_amount, share_price, true);
        let property_price = share_price.saturating_mul((share_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        let payment_asset = T::AcceptedAssets::get()[0];
        let buyer: T::AccountId = account("buyer", 0, 0);
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            buyer.clone(),
            Role::RealEstateInvestor
        ));
        add_buyers_to_listing::<T>(share_amount - 1, payment_asset, property_price, admin.clone());

        assert_ok!(Marketplace::<T>::buy_property_shares(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
            1,
            payment_asset,
        ));
        let spv_admin: T::AccountId = account("spv_admin", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            spv_admin.clone(),
            Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::<T>::create_spv(RawOrigin::Signed(spv_admin).into(), listing_id,));
        claim_buyers_property_shares::<T>(share_amount - 1, listing_id);

        #[extrinsic_call]
        claim_property_shares(RawOrigin::Signed(buyer.clone()), listing_id);

        assert!(ShareOwner::<T>::get(&buyer, listing_id).is_none());
        assert_eq!(
            OngoingObjectListing::<T>::get(listing_id)
                .unwrap()
                .investor_funds
                .get(&buyer)
                .clone()
                .unwrap()
                .paid_funds
                .get(&payment_asset),
            Some(share_price).as_ref()
        );
    }

    #[benchmark]
    fn finalize_claim_window() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());

        let share_amount: u32 = <T as pallet::Config>::MaxPropertyShares::get();
        let share_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let listing_id =
            list_property_helper::<T>(seller, region_id, location, share_amount, share_price, true);
        let property_price = share_price.saturating_mul((share_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        let payment_asset = T::AcceptedAssets::get()[0];
        let buyer: T::AccountId = account("buyer", 0, 0);
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            buyer.clone(),
            Role::RealEstateInvestor
        ));
        add_buyers_to_listing::<T>(share_amount - 1, payment_asset, property_price, admin.clone());

        assert_ok!(Marketplace::<T>::buy_property_shares(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
            1,
            payment_asset,
        ));
        let spv_admin: T::AccountId = account("spv_admin", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            spv_admin.clone(),
            Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::<T>::create_spv(RawOrigin::Signed(spv_admin).into(), listing_id,));
        claim_buyers_property_shares::<T>(share_amount - 1, listing_id);

        let expiry =
            frame_system::Pallet::<T>::block_number() + T::ClaimWindow::get() + 1u32.into();
        frame_system::Pallet::<T>::set_block_number(expiry);
        assert_ok!(Marketplace::<T>::finalize_claim_window(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
        ));
        assert_ok!(Marketplace::<T>::withdraw_unclaimed(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
        ));
        assert_ok!(Marketplace::<T>::buy_property_shares(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
            1,
            payment_asset,
        ));
        let expiry =
            frame_system::Pallet::<T>::block_number() + T::ClaimWindow::get() + 1u32.into();
        frame_system::Pallet::<T>::set_block_number(expiry);

        #[extrinsic_call]
        finalize_claim_window(RawOrigin::Signed(buyer.clone()), listing_id);

        assert_eq!(OngoingObjectListing::<T>::get(listing_id).unwrap().claim_expiry, None);
        assert_eq!(RefundClaimedShare::<T>::get(listing_id), Some(share_amount - 1));
    }

    #[benchmark]
    fn create_spv() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());

        let share_amount: u32 = <T as pallet::Config>::MaxPropertyShares::get();
        let share_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let listing_id =
            list_property_helper::<T>(seller, region_id, location, share_amount, share_price, true);
        let property_price = share_price.saturating_mul((share_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        let payment_asset = T::AcceptedAssets::get()[0];
        let buyer: T::AccountId = account("buyer", 0, 0);
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            buyer.clone(),
            Role::RealEstateInvestor
        ));
        add_buyers_to_listing::<T>(share_amount - 1, payment_asset, property_price, admin.clone());

        assert_ok!(Marketplace::<T>::buy_property_shares(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
            1,
            payment_asset,
        ));

        let spv_admin: T::AccountId = account("spv_admin", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            spv_admin.clone(),
            Role::SpvConfirmation
        ));

        #[extrinsic_call]
        create_spv(RawOrigin::Signed(spv_admin.clone()), listing_id);

        assert!(T::PropertyShares::get_property_asset_info(0).unwrap().spv_created);
    }

    #[benchmark]
    fn relist_shares() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_owner =
            create_registered_property::<T>(seller.clone(), region_id, location, admin);

        let asset_id = 0;
        let amount = 1;
        let price = 5_000u32.into();

        #[extrinsic_call]
        relist_shares(RawOrigin::Signed(share_owner.clone()), asset_id, price, amount);

        let listing = ShareListings::<T>::get(1).unwrap();
        assert_eq!(listing.seller, share_owner);
        assert_eq!(listing.share_price, price);
        assert_eq!(listing.amount, amount);
        assert_eq!(NextListingId::<T>::get(), 2);
    }

    #[benchmark]
    fn buy_relisted_shares() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_owner =
            create_registered_property::<T>(seller.clone(), region_id, location, admin.clone());

        let asset_id = 0;
        let amount = 1;
        let price = 5_000u32.into();

        assert_ok!(Marketplace::<T>::relist_shares(
            RawOrigin::Signed(share_owner.clone()).into(),
            asset_id,
            price,
            amount
        ));

        let payment_asset = T::AcceptedAssets::get()[0];
        let relist_buyer: T::AccountId = account("relist_buyer", 0, 0);
        let deposit_amount = price.saturating_mul(T::ListingDeposit::get());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &relist_buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &relist_buyer,
            price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            relist_buyer.clone(),
            Role::RealEstateInvestor
        ));

        #[extrinsic_call]
        buy_relisted_shares(RawOrigin::Signed(relist_buyer.clone()), 1, amount, payment_asset);

        assert!(!ShareListings::<T>::contains_key(1));
        assert!(T::PropertyShares::get_property_owner(asset_id).contains(&relist_buyer));
        assert!(!T::PropertyShares::get_property_owner(asset_id).contains(&share_owner));
        assert_eq!(T::PropertyShares::get_share_balance(asset_id, &relist_buyer), amount);
        assert_eq!(T::PropertyShares::get_share_balance(asset_id, &share_owner), 0);
    }

    #[benchmark]
    fn cancel_property_purchase() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_amount: u32 = <T as pallet::Config>::MaxPropertyShares::get();
        let share_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let listing_id =
            list_property_helper::<T>(seller, region_id, location, share_amount, share_price, true);
        let property_price = share_price.saturating_mul((share_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        let payment_asset = T::AcceptedAssets::get()[0];
        add_buyers_to_listing::<T>(share_amount - 2, payment_asset, property_price, admin.clone());

        let buyer: T::AccountId = account("buyer_final", 0, 0);
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            buyer.clone(),
            Role::RealEstateInvestor
        ));
        let amount = 1;

        assert_ok!(Marketplace::<T>::buy_property_shares(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
            amount,
            payment_asset,
        ));

        assert_eq!(OngoingObjectListing::<T>::get(listing_id).unwrap().listed_share_amount, 1);

        #[extrinsic_call]
        cancel_property_purchase(RawOrigin::Signed(buyer.clone()), 0);

        assert_eq!(OngoingObjectListing::<T>::get(listing_id).unwrap().listed_share_amount, 2);
    }

    #[benchmark]
    fn make_offer() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_owner =
            create_registered_property::<T>(seller.clone(), region_id, location, admin.clone());

        let asset_id = 0;
        let amount = 1;
        let price = 5_000u32.into();

        assert_ok!(Marketplace::<T>::relist_shares(
            RawOrigin::Signed(share_owner.clone()).into(),
            asset_id,
            price,
            amount
        ));

        let payment_asset = T::AcceptedAssets::get()[0];
        let offerer: T::AccountId = account("offerer", 0, 0);
        let deposit_amount = price.saturating_mul(T::ListingDeposit::get());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &offerer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &offerer,
            price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            offerer.clone(),
            Role::RealEstateInvestor
        ));

        let offer_price = price - 1u32.into();

        #[extrinsic_call]
        make_offer(RawOrigin::Signed(offerer.clone()), 1, offer_price, amount, payment_asset);

        assert_eq!(OngoingOffers::<T>::get(1, offerer).unwrap().share_price, offer_price);
    }

    #[benchmark]
    fn handle_offer() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_owner =
            create_registered_property::<T>(seller.clone(), region_id, location, admin.clone());

        let asset_id = 0;
        let amount = 1;
        let price = 5_000u32.into();

        assert_ok!(Marketplace::<T>::relist_shares(
            RawOrigin::Signed(share_owner.clone()).into(),
            asset_id,
            price,
            amount
        ));

        let payment_asset = T::AcceptedAssets::get()[0];
        let offerer: T::AccountId = account("offerer", 0, 0);
        let deposit_amount = price.saturating_mul(T::ListingDeposit::get());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &offerer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &offerer,
            price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            offerer.clone(),
            Role::RealEstateInvestor
        ));

        let offer_price = price - 1u32.into();

        assert_ok!(Marketplace::<T>::make_offer(
            RawOrigin::Signed(offerer.clone()).into(),
            1,
            offer_price,
            amount,
            payment_asset
        ));

        #[extrinsic_call]
        handle_offer(RawOrigin::Signed(share_owner), 1, offerer.clone(), Offer::Accept, 0);

        assert_eq!(OngoingOffers::<T>::get(1, offerer).is_none(), true);
    }

    #[benchmark]
    fn cancel_offer() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_owner =
            create_registered_property::<T>(seller.clone(), region_id, location, admin.clone());

        let asset_id = 0;
        let amount = 1;
        let price = 5_000u32.into();

        assert_ok!(Marketplace::<T>::relist_shares(
            RawOrigin::Signed(share_owner.clone()).into(),
            asset_id,
            price,
            amount
        ));

        let payment_asset = T::AcceptedAssets::get()[0];
        let offerer: T::AccountId = account("offerer", 0, 0);
        let deposit_amount = price.saturating_mul(T::ListingDeposit::get());
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &offerer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &offerer,
            price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            offerer.clone(),
            Role::RealEstateInvestor
        ));

        let offer_price = price - 1u32.into();

        assert_ok!(Marketplace::<T>::make_offer(
            RawOrigin::Signed(offerer.clone()).into(),
            1,
            offer_price,
            amount,
            payment_asset
        ));

        #[extrinsic_call]
        cancel_offer(RawOrigin::Signed(offerer.clone()), 1);

        assert_eq!(OngoingOffers::<T>::get(1, offerer).is_none(), true);
        assert!(ShareListings::<T>::contains_key(1));
    }

    #[benchmark]
    fn withdraw_rejected() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_amount: u32 = <T as pallet::Config>::MaxPropertyShares::get();
        let share_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let listing_id = list_property_helper::<T>(
            seller.clone(),
            region_id,
            location,
            share_amount,
            share_price,
            true,
        );
        let property_price = share_price.saturating_mul((share_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();

        let payment_asset = T::AcceptedAssets::get()[0];
        let buyer: T::AccountId = account("buyer", 0, 0);
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            buyer.clone(),
            Role::RealEstateInvestor
        ));
        let base = share_amount / 3;
        let remainder = share_amount % 3;
        for i in 1..=2 {
            let buyer_helper: T::AccountId = account("buyer_helper", i, i);
            assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
                &buyer_helper,
                deposit_amount.saturating_mul(20u32.into())
            ));
            assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
                payment_asset,
                &buyer_helper,
                property_price.saturating_mul(100u32.into())
            ));
            assert_ok!(Whitelist::<T>::assign_role(
                RawOrigin::Signed(admin.clone()).into(),
                buyer_helper.clone(),
                Role::RealEstateInvestor
            ));
            assert_ok!(Marketplace::<T>::buy_property_shares(
                RawOrigin::Signed(buyer_helper.clone()).into(),
                listing_id,
                base,
                payment_asset,
            ));
        }
        let buyer_amount = base + remainder;
        assert_ok!(Marketplace::<T>::buy_property_shares(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
            buyer_amount,
            payment_asset,
        ));
        let spv_admin: T::AccountId = account("spv_admin", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            spv_admin.clone(),
            Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::<T>::create_spv(RawOrigin::Signed(spv_admin).into(), listing_id,));
        assert_ok!(Marketplace::<T>::claim_property_shares(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
        ));
        assert_ok!(Marketplace::<T>::claim_property_shares(
            RawOrigin::Signed(account("buyer_helper", 1, 1)).into(),
            listing_id,
        ));
        assert_ok!(Marketplace::<T>::claim_property_shares(
            RawOrigin::Signed(account("buyer_helper", 2, 2)).into(),
            listing_id,
        ));

        let (lawyer_1, lawyer_2) = setup_spv_and_lawyers::<T>(
            seller,
            admin.clone(),
            listing_id,
            region_id,
            buyer.clone(),
            vec![account("buyer_helper", 1, 1), account("buyer_helper", 2, 2)],
        );

        assert_ok!(Marketplace::<T>::unfreeze_spv_lawyer_shares(
            RawOrigin::Signed(account("buyer_helper", 1, 1)).into(),
            0,
        ));
        assert_ok!(Marketplace::<T>::unfreeze_spv_lawyer_shares(
            RawOrigin::Signed(account("buyer_helper", 2, 2)).into(),
            0,
        ));
        assert_ok!(Marketplace::<T>::lawyer_confirm_documents(
            RawOrigin::Signed(lawyer_1).into(),
            0,
            false
        ));
        assert_ok!(Marketplace::<T>::lawyer_confirm_documents(
            RawOrigin::Signed(lawyer_2).into(),
            0,
            false
        ));

        assert_ok!(Marketplace::<T>::withdraw_rejected(
            RawOrigin::Signed(account("buyer_helper", 1, 1)).into(),
            listing_id,
        ));
        assert_ok!(Marketplace::<T>::withdraw_rejected(
            RawOrigin::Signed(account("buyer_helper", 2, 2)).into(),
            listing_id,
        ));

        assert!(RefundShare::<T>::get(listing_id).is_some());

        #[extrinsic_call]
        withdraw_rejected(RawOrigin::Signed(buyer.clone()), listing_id);

        //assert_eq!(ShareOwner::<T>::get(&buyer, listing_id).share_amount, 0);
        assert!(RefundShare::<T>::get(listing_id).is_none());
        assert!(ListingDeposits::<T>::get(listing_id).is_none());
    }

    #[benchmark]
    fn withdraw_legal_process_expired() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_amount: u32 = <T as pallet::Config>::MaxPropertyShares::get();
        let share_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let listing_id = list_property_helper::<T>(
            seller.clone(),
            region_id,
            location,
            share_amount,
            share_price,
            true,
        );
        let property_price = share_price.saturating_mul((share_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        let payment_asset = T::AcceptedAssets::get()[0];
        let buyer: T::AccountId = account("buyer", 0, 0);
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            buyer.clone(),
            Role::RealEstateInvestor
        ));
        let base = share_amount / 3;
        let remainder = share_amount % 3;
        for i in 1..=2 {
            let buyer_helper: T::AccountId = account("buyer_helper", i, i);
            assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
                &buyer_helper,
                deposit_amount.saturating_mul(20u32.into())
            ));
            assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
                payment_asset,
                &buyer_helper,
                property_price.saturating_mul(100u32.into())
            ));
            assert_ok!(Whitelist::<T>::assign_role(
                RawOrigin::Signed(admin.clone()).into(),
                buyer_helper.clone(),
                Role::RealEstateInvestor
            ));
            assert_ok!(Marketplace::<T>::buy_property_shares(
                RawOrigin::Signed(buyer_helper.clone()).into(),
                listing_id,
                base,
                payment_asset,
            ));
        }
        let buyer_amount = base + remainder;
        assert_ok!(Marketplace::<T>::buy_property_shares(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
            buyer_amount,
            payment_asset,
        ));
        let spv_admin: T::AccountId = account("spv_admin", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            spv_admin.clone(),
            Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::<T>::create_spv(RawOrigin::Signed(spv_admin).into(), listing_id,));
        assert_ok!(Marketplace::<T>::claim_property_shares(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
        ));
        assert_ok!(Marketplace::<T>::claim_property_shares(
            RawOrigin::Signed(account("buyer_helper", 1, 1)).into(),
            listing_id,
        ));
        assert_ok!(Marketplace::<T>::claim_property_shares(
            RawOrigin::Signed(account("buyer_helper", 2, 2)).into(),
            listing_id,
        ));

        let (_, _) = setup_spv_and_lawyers::<T>(
            seller,
            admin.clone(),
            listing_id,
            region_id,
            buyer.clone(),
            vec![account("buyer_helper", 1, 1), account("buyer_helper", 2, 2)],
        );

        assert_ok!(Marketplace::<T>::unfreeze_spv_lawyer_shares(
            RawOrigin::Signed(account("buyer_helper", 1, 1)).into(),
            0,
        ));
        assert_ok!(Marketplace::<T>::unfreeze_spv_lawyer_shares(
            RawOrigin::Signed(account("buyer_helper", 2, 2)).into(),
            0,
        ));

        let expiry =
            frame_system::Pallet::<T>::block_number() + T::ClaimWindow::get() + 1u32.into();
        frame_system::Pallet::<T>::set_block_number(expiry);

        assert_ok!(Marketplace::<T>::withdraw_legal_process_expired(
            RawOrigin::Signed(account("buyer_helper", 1, 1)).into(),
            listing_id,
        ));
        assert_ok!(Marketplace::<T>::withdraw_legal_process_expired(
            RawOrigin::Signed(account("buyer_helper", 2, 2)).into(),
            listing_id,
        ));

        assert!(RefundLegalExpired::<T>::get(listing_id).is_some());

        #[extrinsic_call]
        withdraw_legal_process_expired(RawOrigin::Signed(buyer.clone()), listing_id);

        assert!(RefundLegalExpired::<T>::get(listing_id).is_none());
        assert!(ListingDeposits::<T>::get(listing_id).is_none());
    }

    #[benchmark]
    fn withdraw_expired() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_amount: u32 = <T as pallet::Config>::MaxPropertyShares::get();
        let share_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let listing_id =
            list_property_helper::<T>(seller, region_id, location, share_amount, share_price, true);
        let property_price = share_price.saturating_mul((share_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();

        let payment_asset = T::AcceptedAssets::get()[0];
        let buyer: T::AccountId = account("buyer", 0, 0);
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            buyer.clone(),
            Role::RealEstateInvestor
        ));

        assert_ok!(Marketplace::<T>::buy_property_shares(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
            10,
            payment_asset,
        ));

        let expiry =
            frame_system::Pallet::<T>::block_number() + T::MaxListingDuration::get() + 1u32.into();
        frame_system::Pallet::<T>::set_block_number(expiry);

        #[extrinsic_call]
        withdraw_expired(RawOrigin::Signed(buyer.clone()), listing_id);

        //assert_eq!(ShareOwner::<T>::get(&buyer, listing_id).share_amount, 0);
        assert!(OngoingObjectListing::<T>::get(listing_id).is_none());
        assert!(ListingDeposits::<T>::get(listing_id).is_none());
    }

    #[benchmark]
    fn withdraw_deposit_unsold() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_amount: u32 = <T as pallet::Config>::MaxPropertyShares::get();
        let share_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let listing_id = list_property_helper::<T>(
            seller.clone(),
            region_id,
            location,
            share_amount,
            share_price,
            true,
        );

        let expiry =
            frame_system::Pallet::<T>::block_number() + T::MaxListingDuration::get() + 1u32.into();
        frame_system::Pallet::<T>::set_block_number(expiry);

        #[extrinsic_call]
        withdraw_deposit_unsold(RawOrigin::Signed(seller.clone()), listing_id);

        assert!(OngoingObjectListing::<T>::get(listing_id).is_none());
        assert!(ListingDeposits::<T>::get(listing_id).is_none());
        assert!(OngoingObjectListing::<T>::get(listing_id).is_none());
    }

    #[benchmark]
    fn withdraw_claiming_expired() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());

        let share_amount: u32 = <T as pallet::Config>::MaxPropertyShares::get();
        let share_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let listing_id =
            list_property_helper::<T>(seller, region_id, location, share_amount, share_price, true);
        let property_price = share_price.saturating_mul((share_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        let payment_asset = T::AcceptedAssets::get()[0];
        let buyer: T::AccountId = account("buyer", 0, 0);
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            buyer.clone(),
            Role::RealEstateInvestor
        ));
        let base = share_amount / 3;
        let remainder = share_amount % 3;
        for i in 1..=2 {
            let buyer_helper: T::AccountId = account("buyer_helper", i, i);
            assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
                &buyer_helper,
                deposit_amount.saturating_mul(20u32.into())
            ));
            assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
                payment_asset,
                &buyer_helper,
                property_price.saturating_mul(100u32.into())
            ));
            assert_ok!(Whitelist::<T>::assign_role(
                RawOrigin::Signed(admin.clone()).into(),
                buyer_helper.clone(),
                Role::RealEstateInvestor
            ));
            assert_ok!(Marketplace::<T>::buy_property_shares(
                RawOrigin::Signed(buyer_helper.clone()).into(),
                listing_id,
                base,
                payment_asset,
            ));
        }
        let buyer_amount = base + remainder;
        assert_ok!(Marketplace::<T>::buy_property_shares(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
            buyer_amount,
            payment_asset,
        ));

        let spv_admin: T::AccountId = account("spv_admin", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            spv_admin.clone(),
            Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::<T>::create_spv(RawOrigin::Signed(spv_admin).into(), listing_id,));

        assert_ok!(Marketplace::<T>::claim_property_shares(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
        ));

        let expiry =
            frame_system::Pallet::<T>::block_number() + T::ClaimWindow::get() + 1u32.into();
        frame_system::Pallet::<T>::set_block_number(expiry);

        assert_ok!(Marketplace::<T>::finalize_claim_window(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
        ));

        for i in 1..=2 {
            let buyer_helper: T::AccountId = account("buyer_helper", i, i);
            assert_ok!(Marketplace::<T>::withdraw_unclaimed(
                RawOrigin::Signed(buyer_helper.clone()).into(),
                listing_id,
            ));
            assert_ok!(Marketplace::<T>::buy_property_shares(
                RawOrigin::Signed(buyer_helper.clone()).into(),
                listing_id,
                base,
                payment_asset,
            ));
        }

        let expiry =
            frame_system::Pallet::<T>::block_number() + T::ClaimWindow::get() + 1u32.into();
        frame_system::Pallet::<T>::set_block_number(expiry);

        assert_ok!(Marketplace::<T>::finalize_claim_window(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
        ));

        #[extrinsic_call]
        withdraw_claiming_expired(RawOrigin::Signed(buyer.clone()), listing_id);

        assert!(OngoingObjectListing::<T>::get(listing_id).is_none());
        assert!(T::PropertyShares::get_property_asset_info(0).is_none());
    }

    #[benchmark]
    fn withdraw_unclaimed() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());

        let share_amount: u32 = <T as pallet::Config>::MaxPropertyShares::get();
        let share_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let listing_id =
            list_property_helper::<T>(seller, region_id, location, share_amount, share_price, true);
        let property_price = share_price.saturating_mul((share_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();
        let payment_asset = T::AcceptedAssets::get()[0];
        let buyer: T::AccountId = account("buyer", 0, 0);
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &buyer,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &buyer,
            property_price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            buyer.clone(),
            Role::RealEstateInvestor
        ));
        let base = share_amount / 3;
        let remainder = share_amount % 3;
        for i in 1..=2 {
            let buyer_helper: T::AccountId = account("buyer_helper", i, i);
            assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
                &buyer_helper,
                deposit_amount.saturating_mul(20u32.into())
            ));
            assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
                payment_asset,
                &buyer_helper,
                property_price.saturating_mul(100u32.into())
            ));
            assert_ok!(Whitelist::<T>::assign_role(
                RawOrigin::Signed(admin.clone()).into(),
                buyer_helper.clone(),
                Role::RealEstateInvestor
            ));
            assert_ok!(Marketplace::<T>::buy_property_shares(
                RawOrigin::Signed(buyer_helper.clone()).into(),
                listing_id,
                base,
                payment_asset,
            ));
        }
        let buyer_amount = base + remainder;
        assert_ok!(Marketplace::<T>::buy_property_shares(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
            buyer_amount,
            payment_asset,
        ));

        let spv_admin: T::AccountId = account("spv_admin", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            spv_admin.clone(),
            Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::<T>::create_spv(RawOrigin::Signed(spv_admin).into(), listing_id,));

        let expiry =
            frame_system::Pallet::<T>::block_number() + T::ClaimWindow::get() + 1u32.into();
        frame_system::Pallet::<T>::set_block_number(expiry);

        assert_ok!(Marketplace::<T>::finalize_claim_window(
            RawOrigin::Signed(buyer.clone()).into(),
            listing_id,
        ));

        #[extrinsic_call]
        withdraw_unclaimed(RawOrigin::Signed(buyer.clone()), listing_id);

        assert!(ShareOwner::<T>::get(&buyer, listing_id).is_none());
    }

    #[benchmark]
    fn upgrade_object() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_amount: u32 = <T as pallet::Config>::MaxPropertyShares::get();
        let share_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let listing_id = list_property_helper::<T>(
            seller.clone(),
            region_id,
            location,
            share_amount,
            share_price,
            true,
        );

        let new_price = 2_000u32.into();

        #[extrinsic_call]
        upgrade_object(RawOrigin::Signed(seller.clone()), listing_id, new_price);

        assert_eq!(OngoingObjectListing::<T>::get(listing_id).unwrap().share_price, new_price);
    }

    #[benchmark]
    fn delist_shares() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_owner =
            create_registered_property::<T>(seller.clone(), region_id, location, admin);

        let asset_id = 0;
        let amount = 1;
        let price = 5_000u32.into();

        assert_ok!(Marketplace::<T>::relist_shares(
            RawOrigin::Signed(share_owner.clone()).into(),
            asset_id,
            price,
            amount
        ));

        #[extrinsic_call]
        delist_shares(RawOrigin::Signed(share_owner.clone()), 1);

        assert!(!ShareListings::<T>::contains_key(1));
        assert!(T::PropertyShares::get_property_owner(asset_id).contains(&share_owner));
    }

    #[benchmark]
    fn lawyer_claim_property() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let _ =
            list_and_sell_property::<T>(seller.clone(), region_id, location.clone(), admin.clone());

        let lawyer: T::AccountId = account("lawyer", 0, 0);
        let lawyer_deposit = <T as pallet_regions::Config>::LawyerDeposit::get();
        let _ = <T as pallet_regions::Config>::NativeCurrency::mint_into(
            &lawyer,
            lawyer_deposit * 10u32.into(),
        );
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            lawyer.clone(),
            Role::Lawyer
        ));
        assert_ok!(Regions::<T>::register_lawyer(
            RawOrigin::Signed(lawyer.clone()).into(),
            region_id,
        ));

        #[extrinsic_call]
        lawyer_claim_property(
            RawOrigin::Signed(lawyer.clone()),
            0,
            crate::LegalProperty::SpvSide,
            400_u32.into(),
        );

        assert_eq!(SpvLawyerProposal::<T>::get(0).unwrap().lawyer, lawyer);
    }

    #[benchmark]
    fn vote_on_spv_lawyer() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_holder =
            list_and_sell_property::<T>(seller.clone(), region_id, location.clone(), admin.clone());

        let lawyer: T::AccountId = account("lawyer", 0, 0);
        let lawyer_deposit = <T as pallet_regions::Config>::LawyerDeposit::get();
        let _ = <T as pallet_regions::Config>::NativeCurrency::mint_into(
            &lawyer,
            lawyer_deposit * 10u32.into(),
        );
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            lawyer.clone(),
            Role::Lawyer
        ));
        assert_ok!(Regions::<T>::register_lawyer(
            RawOrigin::Signed(lawyer.clone()).into(),
            region_id,
        ));

        assert_ok!(Marketplace::<T>::lawyer_claim_property(
            RawOrigin::Signed(lawyer.clone()).into(),
            0,
            crate::LegalProperty::SpvSide,
            400_u32.into(),
        ));

        for i in 1..<T as pallet::Config>::MaxPropertyShares::get() {
            let buyer: T::AccountId = account("buyer", i, i);
            assert_ok!(Marketplace::<T>::vote_on_spv_lawyer(
                RawOrigin::Signed(buyer.clone()).into(),
                0,
                crate::Vote::Yes,
                1
            ));
            assert!(UserLawyerVote::<T>::get(0, &buyer).is_some());
        }

        assert_ok!(Marketplace::<T>::vote_on_spv_lawyer(
            RawOrigin::Signed(share_holder.clone()).into(),
            0,
            crate::Vote::No,
            1
        ));
        assert_eq!(UserLawyerVote::<T>::get(0, &share_holder).unwrap().vote, crate::Vote::No);

        #[extrinsic_call]
        vote_on_spv_lawyer(RawOrigin::Signed(share_holder.clone()), 0, types::Vote::Yes, 1);

        assert_eq!(SpvLawyerProposal::<T>::get(0).unwrap().lawyer, lawyer);
        assert_eq!(
            OngoingLawyerVoting::<T>::get(0).unwrap().yes_voting_power,
            <T as pallet::Config>::MaxPropertyShares::get()
        );
        assert_eq!(UserLawyerVote::<T>::get(0, share_holder).unwrap().vote, crate::Vote::Yes);
    }

    #[benchmark]
    fn approve_developer_lawyer() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let _ =
            list_and_sell_property::<T>(seller.clone(), region_id, location.clone(), admin.clone());

        let lawyer: T::AccountId = account("lawyer", 0, 0);
        let lawyer_deposit = <T as pallet_regions::Config>::LawyerDeposit::get();
        let _ = <T as pallet_regions::Config>::NativeCurrency::mint_into(
            &lawyer,
            lawyer_deposit * 10u32.into(),
        );
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            lawyer.clone(),
            Role::Lawyer
        ));
        assert_ok!(Regions::<T>::register_lawyer(
            RawOrigin::Signed(lawyer.clone()).into(),
            region_id,
        ));

        assert_ok!(Marketplace::<T>::lawyer_claim_property(
            RawOrigin::Signed(lawyer.clone()).into(),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            400_u32.into(),
        ));

        #[extrinsic_call]
        approve_developer_lawyer(RawOrigin::Signed(seller), 0, true);

        assert!(ProposedLawyers::<T>::get(0).is_none());
        assert_eq!(
            PropertyLawyer::<T>::get(0).unwrap().real_estate_developer_lawyer,
            Some(lawyer.clone())
        );
    }

    #[benchmark]
    fn finalize_spv_lawyer() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_holder =
            list_and_sell_property::<T>(seller.clone(), region_id, location.clone(), admin.clone());

        let lawyer: T::AccountId = account("lawyer", 0, 0);
        let lawyer_deposit = <T as pallet_regions::Config>::LawyerDeposit::get();
        let _ = <T as pallet_regions::Config>::NativeCurrency::mint_into(
            &lawyer,
            lawyer_deposit * 10u32.into(),
        );
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            lawyer.clone(),
            Role::Lawyer
        ));
        assert_ok!(Regions::<T>::register_lawyer(
            RawOrigin::Signed(lawyer.clone()).into(),
            region_id,
        ));

        assert_ok!(Marketplace::<T>::lawyer_claim_property(
            RawOrigin::Signed(lawyer.clone()).into(),
            0,
            crate::LegalProperty::SpvSide,
            400_u32.into(),
        ));

        assert_ok!(Marketplace::<T>::vote_on_spv_lawyer(
            RawOrigin::Signed(share_holder.clone()).into(),
            0,
            types::Vote::Yes,
            1
        ));
        for i in 1..<T as pallet::Config>::MaxPropertyShares::get() {
            let buyer: T::AccountId = account("buyer", i, i);
            assert_ok!(Marketplace::<T>::vote_on_spv_lawyer(
                RawOrigin::Signed(buyer.clone()).into(),
                0,
                crate::Vote::Yes,
                1
            ));
            assert!(UserLawyerVote::<T>::get(0, &buyer).is_some());
        }
        let expiry = frame_system::Pallet::<T>::block_number() + T::LawyerVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);

        #[extrinsic_call]
        finalize_spv_lawyer(RawOrigin::Signed(share_holder.clone()), 0);

        assert!(SpvLawyerProposal::<T>::get(0).is_none());
        assert!(OngoingLawyerVoting::<T>::get(0).is_none());
        assert_eq!(PropertyLawyer::<T>::get(0).unwrap().spv_lawyer, Some(lawyer.clone()));
        assert!(UserLawyerVote::<T>::get(0, &share_holder).is_some());
    }

    #[benchmark]
    fn unfreeze_spv_lawyer_shares() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_holder =
            list_and_sell_property::<T>(seller.clone(), region_id, location.clone(), admin.clone());

        let lawyer: T::AccountId = account("lawyer", 0, 0);
        let lawyer_deposit = <T as pallet_regions::Config>::LawyerDeposit::get();
        let _ = <T as pallet_regions::Config>::NativeCurrency::mint_into(
            &lawyer,
            lawyer_deposit * 10u32.into(),
        );
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            lawyer.clone(),
            Role::Lawyer
        ));
        assert_ok!(Regions::<T>::register_lawyer(
            RawOrigin::Signed(lawyer.clone()).into(),
            region_id,
        ));

        assert_ok!(Marketplace::<T>::lawyer_claim_property(
            RawOrigin::Signed(lawyer.clone()).into(),
            0,
            crate::LegalProperty::SpvSide,
            400_u32.into(),
        ));

        assert_ok!(Marketplace::<T>::vote_on_spv_lawyer(
            RawOrigin::Signed(share_holder.clone()).into(),
            0,
            types::Vote::Yes,
            1
        ));
        for i in 1..<T as pallet::Config>::MaxPropertyShares::get() {
            let buyer: T::AccountId = account("buyer", i, i);
            assert_ok!(Marketplace::<T>::vote_on_spv_lawyer(
                RawOrigin::Signed(buyer.clone()).into(),
                0,
                crate::Vote::Yes,
                1
            ));
            assert!(UserLawyerVote::<T>::get(0, &buyer).is_some());
        }
        let expiry = frame_system::Pallet::<T>::block_number() + T::LawyerVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);

        assert_ok!(Marketplace::<T>::finalize_spv_lawyer(
            RawOrigin::Signed(share_holder.clone()).into(),
            0,
        ));

        assert!(UserLawyerVote::<T>::get(0, &share_holder).is_some());

        #[extrinsic_call]
        unfreeze_spv_lawyer_shares(RawOrigin::Signed(share_holder.clone()), 0);

        assert!(UserLawyerVote::<T>::get(0, &share_holder).is_none());
    }

    #[benchmark]
    fn remove_lawyer_claim() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_holder =
            list_and_sell_property::<T>(seller.clone(), region_id, location.clone(), admin.clone());

        let lawyer_1: T::AccountId = account("lawyer1", 0, 0);
        let lawyer_2: T::AccountId = account("lawyer2", 0, 0);

        let lawyer_deposit = <T as pallet_regions::Config>::LawyerDeposit::get();
        let _ = <T as pallet_regions::Config>::NativeCurrency::mint_into(
            &lawyer_1,
            lawyer_deposit * 10u32.into(),
        );
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            lawyer_1.clone(),
            Role::Lawyer
        ));
        assert_ok!(Regions::<T>::register_lawyer(
            RawOrigin::Signed(lawyer_1.clone()).into(),
            region_id,
        ));
        let lawyer_deposit = <T as pallet_regions::Config>::LawyerDeposit::get();
        let _ = <T as pallet_regions::Config>::NativeCurrency::mint_into(
            &lawyer_2,
            lawyer_deposit * 10u32.into(),
        );
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            lawyer_2.clone(),
            Role::Lawyer
        ));
        assert_ok!(Regions::<T>::register_lawyer(
            RawOrigin::Signed(lawyer_2.clone()).into(),
            region_id,
        ));
        assert_ok!(Marketplace::<T>::lawyer_claim_property(
            RawOrigin::Signed(lawyer_1.clone()).into(),
            0,
            crate::LegalProperty::RealEstateDeveloperSide,
            400_u32.into()
        ));
        assert_ok!(Marketplace::<T>::approve_developer_lawyer(
            RawOrigin::Signed(seller).into(),
            0,
            true
        ));
        assert_ok!(Marketplace::<T>::lawyer_claim_property(
            RawOrigin::Signed(lawyer_2.clone()).into(),
            0,
            crate::LegalProperty::SpvSide,
            400_u32.into()
        ));
        assert_ok!(Marketplace::<T>::vote_on_spv_lawyer(
            RawOrigin::Signed(share_holder.clone()).into(),
            0,
            types::Vote::Yes,
            1
        ));
        for i in 1..<T as pallet::Config>::MaxPropertyShares::get() {
            let buyer: T::AccountId = account("buyer", i, i);
            assert_ok!(Marketplace::<T>::vote_on_spv_lawyer(
                RawOrigin::Signed(buyer.clone()).into(),
                0,
                crate::Vote::Yes,
                1
            ));
            assert!(UserLawyerVote::<T>::get(0, &buyer).is_some());
        }
        let expiry = frame_system::Pallet::<T>::block_number() + T::LawyerVotingTime::get();
        frame_system::Pallet::<T>::set_block_number(expiry);
        assert_ok!(Marketplace::<T>::finalize_spv_lawyer(
            RawOrigin::Signed(share_holder).into(),
            0,
        ));

        #[extrinsic_call]
        remove_lawyer_claim(RawOrigin::Signed(lawyer_2.clone()), 0);

        assert_eq!(PropertyLawyer::<T>::get(0).unwrap().spv_lawyer, None);
    }

    #[benchmark]
    fn lawyer_confirm_documents(a: Linear<1, { <T as pallet::Config>::MaxPropertyShares::get() }>) {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_amount: u32 = <T as pallet::Config>::MaxPropertyShares::get();
        let share_price: <T as pallet::Config>::Balance = 1_000u32.into();
        let listing_id = list_property_helper::<T>(
            seller.clone(),
            region_id,
            location,
            share_amount,
            share_price,
            true,
        );
        let property_price = share_price.saturating_mul((share_amount as u128).into());
        let deposit_amount =
            property_price.saturating_mul(T::ListingDeposit::get()) / 100u128.into();

        let payment_asset = T::AcceptedAssets::get()[0];
        let share_holder: T::AccountId = account("buyer", 0, 0);
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &share_holder,
            deposit_amount.saturating_mul(20u32.into())
        ));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            payment_asset,
            &share_holder,
            property_price.saturating_mul(100u32.into())
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            share_holder.clone(),
            Role::RealEstateInvestor
        ));
        add_buyers_to_listing::<T>(a - 1, payment_asset, property_price, admin.clone());

        let buyer_amount = if share_amount - a + 1 >= share_amount * 50 / 100 {
            let base = (share_amount - a + 1) / 3;
            let remainder = (share_amount - a + 1) % 3;
            for i in 1..=2 {
                let buyer_helper: T::AccountId = account("buyer_helper", i, i);
                assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
                    &buyer_helper,
                    deposit_amount.saturating_mul(20u32.into())
                ));
                assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
                    payment_asset,
                    &buyer_helper,
                    property_price.saturating_mul(100u32.into())
                ));
                assert_ok!(Whitelist::<T>::assign_role(
                    RawOrigin::Signed(admin.clone()).into(),
                    buyer_helper.clone(),
                    Role::RealEstateInvestor
                ));
                assert_ok!(Marketplace::<T>::buy_property_shares(
                    RawOrigin::Signed(buyer_helper.clone()).into(),
                    listing_id,
                    base,
                    payment_asset,
                ));
            }
            base + remainder
        } else {
            share_amount - a + 1
        };
        assert_ok!(Marketplace::<T>::buy_property_shares(
            RawOrigin::Signed(share_holder.clone()).into(),
            listing_id,
            buyer_amount,
            payment_asset,
        ));
        let spv_admin: T::AccountId = account("spv_admin", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            spv_admin.clone(),
            Role::SpvConfirmation
        ));
        assert_ok!(Marketplace::<T>::create_spv(RawOrigin::Signed(spv_admin).into(), listing_id,));
        assert_ok!(Marketplace::<T>::claim_property_shares(
            RawOrigin::Signed(share_holder.clone()).into(),
            listing_id,
        ));
        claim_buyers_property_shares::<T>(a - 1, listing_id);
        if share_amount - a + 1 >= share_amount * 50 / 100 {
            for i in 1..=2 {
                let buyer_helper: T::AccountId = account("buyer_helper", i, i);
                assert_ok!(Marketplace::<T>::claim_property_shares(
                    RawOrigin::Signed(buyer_helper.clone()).into(),
                    listing_id,
                ));
            }
        }

        let mut share_holders: Vec<T::AccountId> = vec![share_holder.clone()];
        for i in 1..a - 1 {
            let buyer: T::AccountId = account("buyer", i, i);
            share_holders.push(buyer);
        }
        if share_amount - a + 1 >= share_amount * 50 / 100 {
            let buyer_helper: T::AccountId = account("buyer_helper", 1, 1);
            share_holders.push(buyer_helper);
        }
        let (lawyer_1, lawyer_2) = setup_spv_and_lawyers::<T>(
            seller,
            admin.clone(),
            listing_id,
            region_id,
            share_holder.clone(),
            share_holders,
        );

        assert_ok!(Marketplace::<T>::lawyer_confirm_documents(
            RawOrigin::Signed(lawyer_1).into(),
            listing_id,
            true
        ));

        #[extrinsic_call]
        lawyer_confirm_documents(RawOrigin::Signed(lawyer_2.clone()), listing_id, true);

        assert!(PropertyLawyer::<T>::get(listing_id).is_none());
        assert!(OngoingObjectListing::<T>::get(listing_id).is_none());
    }

    #[benchmark]
    fn send_property_shares() {
        let (seller, admin): (T::AccountId, T::AccountId) = create_whitelisted_user::<T>();
        let (region_id, location) = create_a_new_region::<T>(seller.clone(), admin.clone());
        let share_owner =
            create_registered_property::<T>(seller.clone(), region_id, location, admin.clone());

        let new_owner: T::AccountId = account("new_owner", 0, 0);
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin).into(),
            new_owner.clone(),
            Role::RealEstateInvestor
        ));
        let deposit_amount = T::ListingDeposit::get();

        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(
            &new_owner,
            deposit_amount.saturating_mul(100_000_000u32.into())
        ));

        let asset_id = 0;
        let share_amount = 1;

        #[extrinsic_call]
        send_property_shares(
            RawOrigin::Signed(share_owner.clone()),
            asset_id,
            new_owner.clone(),
            share_amount,
        );

        assert_eq!(T::PropertyShares::get_share_balance(asset_id, &seller), 0);
        assert_eq!(T::PropertyShares::get_share_balance(asset_id, &new_owner.clone()), share_amount);
        assert!(T::PropertyShares::get_property_owner(asset_id).contains(&new_owner));
        assert!(!T::PropertyShares::get_property_owner(asset_id).contains(&seller));
    }

    impl_benchmark_test_suite!(Marketplace, crate::mock::new_test_ext(), crate::mock::Test);
}
