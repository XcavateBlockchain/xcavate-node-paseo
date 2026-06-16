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

//! Benchmarking setup for pallet-real-x-education
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as RealXEducation;
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_support::traits::{fungible::Mutate, nonfungibles_v2::Inspect};
use frame_system::{Pallet as System, RawOrigin};
use pallet_education_regions::Pallet as Regions;
use pallet_education_regions::{RegionIdentifier, Vote};
use pallet_xcavate_whitelist::Pallet as Whitelist;
use scale_info::prelude::vec;

pub trait Config:
    pallet_xcavate_whitelist::Config + pallet_education_regions::Config + crate::Config
{
}

impl<T: crate::Config + pallet_xcavate_whitelist::Config + pallet_education_regions::Config> Config
    for T
{
}

fn sponsor_mint_amount<T: Config>() -> <T as pallet::Config>::Balance {
    let payment_asset = T::AcceptedAssets::get()[0];
    let decimals = T::AssetMetadata::get_decimals(payment_asset).unwrap_or(18);
    T::ModulePrice::get() * 200u32.into() * 10u128.pow(decimals as u32).into()
}

fn create_a_new_region<T: Config>(admin: T::AccountId) -> u16 {
    let regional_operator = account("regional_operator", 0, 0);
    let region = RegionIdentifier::Japan;
    let region_id = region.clone().into_u16();

    let deposit = T::RegionProposalDeposit::get();
    let auction_amount = T::MinimumRegionDeposit::get();
    let total_funds = deposit
        .saturating_mul(1000u32.into())
        .saturating_add(auction_amount.saturating_mul(100u32.into()));
    assert_ok!(<T as pallet_education_regions::Config>::NativeCurrency::mint_into(
        &regional_operator,
        total_funds
    ));
    assert_ok!(Whitelist::<T>::assign_role(
        RawOrigin::Signed(admin.clone()).into(),
        regional_operator.clone(),
        Role::RegionalOperator
    ));
    assert_ok!(Regions::<T>::propose_new_region(
        RawOrigin::Signed(regional_operator.clone()).into(),
        region.clone()
    ));
    assert_ok!(Regions::<T>::vote_on_region_proposal(
        RawOrigin::Signed(regional_operator.clone()).into(),
        region_id,
        Vote::Yes,
        deposit.saturating_mul(800u32.into())
    ));

    let bid_amount = auction_amount.saturating_mul(10u32.into());

    let expiry = frame_system::Pallet::<T>::block_number() + T::RegionVotingTime::get();
    frame_system::Pallet::<T>::set_block_number(expiry);

    assert_ok!(Regions::<T>::bid_on_region(
        RawOrigin::Signed(regional_operator.clone()).into(),
        region_id,
        bid_amount
    ));

    let auction_expiry = frame_system::Pallet::<T>::block_number() + T::RegionAuctionTime::get();
    frame_system::Pallet::<T>::set_block_number(auction_expiry);
    assert_ok!(Regions::<T>::create_new_region(
        RawOrigin::Signed(regional_operator.clone()).into(),
        region_id,
    ));

    // Verify region and location
    assert!(pallet_education_regions::RegionDetails::<T>::contains_key(region_id));

    region_id
}

fn run_to_block<T: Config>(new_block: frame_system::pallet_prelude::BlockNumberFor<T>) {
    while System::<T>::block_number() < new_block {
        if System::<T>::block_number() > 0u32.into() {
            RealXEducation::<T>::on_initialize(System::<T>::block_number());
            System::<T>::on_finalize(System::<T>::block_number());
        }
        System::<T>::reset_events();
        System::<T>::set_block_number(System::<T>::block_number() + 1u32.into());
        System::<T>::on_initialize(System::<T>::block_number());
        RealXEducation::<T>::on_initialize(System::<T>::block_number());
    }
}

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn create_module() {
        let admin: T::AccountId = account("admin", 0, 0);
        let creator: T::AccountId = account("creator", 0, 0);
        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));
        let region_id = create_a_new_region::<T>(admin.clone());
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            creator.clone(),
            Role::ModuleCreator
        ));

        let deposit = T::BookingDeposit::get() * 100u32.into();
        let _ = <T as pallet::Config>::NativeCurrency::mint_into(&creator, deposit);

        let metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        let module_amount: u32 = 100u32;

        #[extrinsic_call]
        create_module(RawOrigin::Signed(creator.clone()), region_id, module_amount, metadata);

        let module = ModuleInfo::<T>::get(0).expect("Module should exist");

        assert_eq!(module.creator, creator);
        assert_eq!(module.total_token_amount, module_amount);
        assert_eq!(module.sponsor_allocation, module_amount);
        assert_eq!(module.school_allocation, 0);
        assert_eq!(module.university_student_allocation, 0);
        assert_eq!(module.asset_id, 0);
    }

    #[benchmark]
    fn sponsor_module() {
        let admin: T::AccountId = account("admin", 0, 0);
        let creator: T::AccountId = account("creator", 0, 0);
        let sponsor: T::AccountId = account("sponsor", 0, 0);
        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));
        let region_id = create_a_new_region::<T>(admin.clone());
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            creator.clone(),
            Role::ModuleCreator
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            sponsor.clone(),
            Role::ModuleSponsor
        ));

        let deposit = T::BookingDeposit::get() * 100u32.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&creator, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&sponsor, deposit));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            T::AcceptedAssets::get()[0],
            &sponsor,
            sponsor_mint_amount::<T>()
        ));

        let metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        let module_amount: u32 = 100u32;
        assert_ok!(RealXEducation::<T>::create_module(
            RawOrigin::Signed(creator.clone()).into(),
            region_id,
            module_amount,
            metadata
        ));

        #[extrinsic_call]
        sponsor_module(RawOrigin::Signed(sponsor.clone()), 0, 10, T::AcceptedAssets::get()[0]);

        assert_eq!(SponsoredModules::<T>::get(0, 0).unwrap().amount, 10);
        assert_eq!(SponsoredModules::<T>::get(0, 0).unwrap().payment_asset, T::AcceptedAssets::get()[0]);
    }

    #[benchmark]
    fn book_module() {
        let admin: T::AccountId = account("admin", 0, 0);
        let creator: T::AccountId = account("creator", 0, 0);
        let sponsor: T::AccountId = account("sponsor", 0, 0);
        let school: T::AccountId = account("school", 0, 0);
        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));
        let region_id = create_a_new_region::<T>(admin.clone());
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            creator.clone(),
            Role::ModuleCreator
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            sponsor.clone(),
            Role::ModuleSponsor
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            school.clone(),
            Role::ModuleBooker
        ));

        let deposit = T::BookingDeposit::get() * 100u32.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&creator, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&sponsor, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&school, deposit));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            T::AcceptedAssets::get()[0],
            &sponsor,
            sponsor_mint_amount::<T>()
        ));

        let metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        let module_amount: u32 = 100u32;
        assert_ok!(RealXEducation::<T>::create_module(
            RawOrigin::Signed(creator.clone()).into(),
            region_id,
            module_amount,
            metadata
        ));

        assert_ok!(RealXEducation::<T>::sponsor_module(
            RawOrigin::Signed(sponsor.clone()).into(),
            0,
            10,
            T::AcceptedAssets::get()[0]
        ));

        let booking_metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                43u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        #[extrinsic_call]
        book_module(RawOrigin::Signed(school.clone()), 0, 0, booking_metadata);

        let booking = Bookings::<T>::get(0, 0).expect("Booking should exist");
        assert_eq!(booking.sponsor, sponsor);
        assert_eq!(booking.school, school);
        assert_eq!(booking.lecturer, None);
    }

    #[benchmark]
    fn claim_booking() {
        let admin: T::AccountId = account("admin", 0, 0);
        let creator: T::AccountId = account("creator", 0, 0);
        let sponsor: T::AccountId = account("sponsor", 0, 0);
        let school: T::AccountId = account("school", 0, 0);
        let university_student: T::AccountId = account("university_student", 0, 0);
        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));
        let region_id = create_a_new_region::<T>(admin.clone());
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            creator.clone(),
            Role::ModuleCreator
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            sponsor.clone(),
            Role::ModuleSponsor
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            school.clone(),
            Role::ModuleBooker
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            university_student.clone(),
            Role::ModuleDeliverer
        ));

        let deposit = T::BookingDeposit::get() * 100u32.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&creator, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&sponsor, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&school, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&university_student, deposit));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            T::AcceptedAssets::get()[0],
            &sponsor,
            sponsor_mint_amount::<T>()
        ));

        assert_ok!(RealXEducation::<T>::register_module_deliverer(
            RawOrigin::Signed(university_student.clone()).into(),
        ));

        let metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        let module_amount: u32 = 100u32;
        assert_ok!(RealXEducation::<T>::create_module(
            RawOrigin::Signed(creator.clone()).into(),
            region_id,
            module_amount,
            metadata
        ));

        assert_ok!(RealXEducation::<T>::sponsor_module(
            RawOrigin::Signed(sponsor.clone()).into(),
            0,
            10,
            T::AcceptedAssets::get()[0]
        ));

        let booking_metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                43u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        assert_ok!(RealXEducation::<T>::book_module(
            RawOrigin::Signed(school.clone()).into(),
            0,
            0,
            booking_metadata
        ));

        #[extrinsic_call]
        claim_booking(RawOrigin::Signed(university_student.clone()), 0, 0);

        assert_eq!(Bookings::<T>::get(0, 0).unwrap().lecturer, Some(university_student));
    }

    #[benchmark]
    fn submit_impact_score() {
        let admin: T::AccountId = account("admin", 0, 0);
        let creator: T::AccountId = account("creator", 0, 0);
        let sponsor: T::AccountId = account("sponsor", 0, 0);
        let school: T::AccountId = account("school", 0, 0);
        let ai_agent: T::AccountId = account("ai_agent", 0, 0);
        let university_student: T::AccountId = account("university_student", 0, 0);
        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));
        let region_id = create_a_new_region::<T>(admin.clone());
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            creator.clone(),
            Role::ModuleCreator
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            sponsor.clone(),
            Role::ModuleSponsor
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            school.clone(),
            Role::ModuleBooker
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            university_student.clone(),
            Role::ModuleDeliverer
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            ai_agent.clone(),
            Role::ModuleAIAgent
        ));

        let deposit = T::BookingDeposit::get() * 100u32.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&creator, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&sponsor, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&school, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&university_student, deposit));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            T::AcceptedAssets::get()[0],
            &sponsor,
            sponsor_mint_amount::<T>()
        ));

        assert_ok!(RealXEducation::<T>::register_module_deliverer(
            RawOrigin::Signed(university_student.clone()).into(),
        ));

        let metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        let module_amount: u32 = 100u32;
        assert_ok!(RealXEducation::<T>::create_module(
            RawOrigin::Signed(creator.clone()).into(),
            region_id,
            module_amount,
            metadata
        ));

        assert_ok!(RealXEducation::<T>::sponsor_module(
            RawOrigin::Signed(sponsor.clone()).into(),
            0,
            10,
            T::AcceptedAssets::get()[0]
        ));

        let booking_metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                43u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        assert_ok!(RealXEducation::<T>::book_module(
            RawOrigin::Signed(school.clone()).into(),
            0,
            0,
            booking_metadata
        ));
        assert_ok!(RealXEducation::<T>::claim_booking(
            RawOrigin::Signed(university_student.clone()).into(),
            0,
            0
        ));

        let sponsor_metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                44u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);
        let school_metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                45u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);
        let lecturer_metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                46u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        #[extrinsic_call]
        submit_impact_score(
            RawOrigin::Signed(ai_agent.clone()),
            0,
            0,
            Permill::from_percent(75),
            sponsor_metadata,
            school_metadata,
            lecturer_metadata,
        );

        assert_eq!(Bookings::<T>::get(0, 0).unwrap().score, Some(Permill::from_percent(75)));
    }

    #[benchmark]
    fn mint_recipient_nft() {
        let admin: T::AccountId = account("admin", 0, 0);
        let creator: T::AccountId = account("creator", 0, 0);
        let sponsor: T::AccountId = account("sponsor", 0, 0);
        let school: T::AccountId = account("school", 0, 0);
        let ai_agent: T::AccountId = account("ai_agent", 0, 0);
        let university_student: T::AccountId = account("university_student", 0, 0);
        let high_school_student: T::AccountId = account("high_school_student", 0, 0);
        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));
        let region_id = create_a_new_region::<T>(admin.clone());
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            creator.clone(),
            Role::ModuleCreator
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            sponsor.clone(),
            Role::ModuleSponsor
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            school.clone(),
            Role::ModuleBooker
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            university_student.clone(),
            Role::ModuleDeliverer
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            ai_agent.clone(),
            Role::ModuleAIAgent
        ));

        let deposit = T::BookingDeposit::get() * 100u32.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&creator, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&sponsor, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&school, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&university_student, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&high_school_student, deposit));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            T::AcceptedAssets::get()[0],
            &sponsor,
            sponsor_mint_amount::<T>()
        ));

        assert_ok!(RealXEducation::<T>::register_module_deliverer(
            RawOrigin::Signed(university_student.clone()).into(),
        ));

        let metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        let module_amount: u32 = 100u32;
        assert_ok!(RealXEducation::<T>::create_module(
            RawOrigin::Signed(creator.clone()).into(),
            region_id,
            module_amount,
            metadata
        ));

        assert_ok!(RealXEducation::<T>::sponsor_module(
            RawOrigin::Signed(sponsor.clone()).into(),
            0,
            10,
            T::AcceptedAssets::get()[0]
        ));

        let booking_metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                43u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        assert_ok!(RealXEducation::<T>::book_module(
            RawOrigin::Signed(school.clone()).into(),
            0,
            0,
            booking_metadata
        ));
        assert_ok!(RealXEducation::<T>::claim_booking(
            RawOrigin::Signed(university_student.clone()).into(),
            0,
            0
        ));

        let sponsor_metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                44u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);
        let school_metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                45u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);
        let lecturer_metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                46u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        assert_ok!(RealXEducation::<T>::submit_impact_score(
            RawOrigin::Signed(ai_agent.clone()).into(),
            0,
            0,
            Permill::from_percent(75),
            sponsor_metadata,
            school_metadata,
            lecturer_metadata
        ));

        let student_metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                46u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        let module = ModuleInfo::<T>::get(0).expect("Module should exist");
        let item_id = NextNftId::<T>::get(module.collection_id);

        #[extrinsic_call]
        mint_recipient_nft(
            RawOrigin::Signed(ai_agent),
            0,
            0,
            high_school_student.clone(),
            student_metadata,
        );

        assert_eq!(
            <T as pallet::Config>::Nfts::owner(&module.collection_id, &item_id),
            Some(high_school_student)
        );
    }

    #[benchmark]
    fn finish_booking_process() {
        let admin: T::AccountId = account("admin", 0, 0);
        let creator: T::AccountId = account("creator", 0, 0);
        let sponsor: T::AccountId = account("sponsor", 0, 0);
        let school: T::AccountId = account("school", 0, 0);
        let ai_agent: T::AccountId = account("ai_agent", 0, 0);
        let university_student: T::AccountId = account("university_student", 0, 0);
        let high_school_student: T::AccountId = account("high_school_student", 0, 0);
        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));
        let region_id = create_a_new_region::<T>(admin.clone());
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            creator.clone(),
            Role::ModuleCreator
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            sponsor.clone(),
            Role::ModuleSponsor
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            school.clone(),
            Role::ModuleBooker
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            university_student.clone(),
            Role::ModuleDeliverer
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            ai_agent.clone(),
            Role::ModuleAIAgent
        ));

        let deposit = T::BookingDeposit::get() * 100u32.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&creator, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&sponsor, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&school, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&university_student, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&high_school_student, deposit));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            T::AcceptedAssets::get()[0],
            &sponsor,
            sponsor_mint_amount::<T>()
        ));

        assert_ok!(RealXEducation::<T>::register_module_deliverer(
            RawOrigin::Signed(university_student.clone()).into(),
        ));

        let metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        let module_amount: u32 = 100u32;
        assert_ok!(RealXEducation::<T>::create_module(
            RawOrigin::Signed(creator.clone()).into(),
            region_id,
            module_amount,
            metadata
        ));

        assert_ok!(RealXEducation::<T>::sponsor_module(
            RawOrigin::Signed(sponsor.clone()).into(),
            0,
            10,
            T::AcceptedAssets::get()[0]
        ));

        let booking_metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                43u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        assert_ok!(RealXEducation::<T>::book_module(
            RawOrigin::Signed(school.clone()).into(),
            0,
            0,
            booking_metadata
        ));
        assert_ok!(RealXEducation::<T>::claim_booking(
            RawOrigin::Signed(university_student.clone()).into(),
            0,
            0
        ));

        let sponsor_metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                44u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);
        let school_metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                45u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);
        let lecturer_metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                46u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        assert_ok!(RealXEducation::<T>::submit_impact_score(
            RawOrigin::Signed(ai_agent.clone()).into(),
            0,
            0,
            Permill::from_percent(75),
            sponsor_metadata,
            school_metadata,
            lecturer_metadata
        ));

        assert!(Bookings::<T>::get(0, 0).is_some());

        #[extrinsic_call]
        finish_booking_process(RawOrigin::Signed(school), 0, 0);

        assert!(Bookings::<T>::get(0, 0).is_none());
    }

    #[benchmark]
    fn burn_unsponsored_token() {
        let admin: T::AccountId = account("admin", 0, 0);
        let creator: T::AccountId = account("creator", 0, 0);
        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));
        let region_id = create_a_new_region::<T>(admin.clone());
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            creator.clone(),
            Role::ModuleCreator
        ));

        let deposit = T::BookingDeposit::get() * 100u32.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&creator, deposit));

        let metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        let module_amount: u32 = 100u32;
        assert_ok!(RealXEducation::<T>::create_module(
            RawOrigin::Signed(creator.clone()).into(),
            region_id,
            module_amount,
            metadata
        ));

        #[extrinsic_call]
        burn_unsponsored_token(RawOrigin::Signed(creator), 0, 80);

        assert_eq!(T::LocalCurrency::total_issuance(0), 20u32.into());
        assert_eq!(ModuleInfo::<T>::get(0).unwrap().sponsor_allocation, 20);
    }

    #[benchmark]
    fn remove_module() {
        let admin: T::AccountId = account("admin", 0, 0);
        let creator: T::AccountId = account("creator", 0, 0);
        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));
        let region_id = create_a_new_region::<T>(admin.clone());
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            creator.clone(),
            Role::ModuleCreator
        ));

        let deposit = T::BookingDeposit::get() * 100u32.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&creator, deposit));

        let metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        let module_amount: u32 = 100u32;
        assert_ok!(RealXEducation::<T>::create_module(
            RawOrigin::Signed(creator.clone()).into(),
            region_id,
            module_amount,
            metadata
        ));

        assert_ok!(RealXEducation::<T>::burn_unsponsored_token(
            RawOrigin::Signed(creator.clone()).into(),
            0,
            module_amount
        ));

        assert_eq!(T::LocalCurrency::total_issuance(0), 0u32.into());

        #[extrinsic_call]
        remove_module(RawOrigin::Signed(creator), 0);

        assert!(ModuleInfo::<T>::get(0).is_none());
        assert!(!T::LocalCurrency::asset_exists(0));
    }

    #[benchmark]
    fn reclaim_unused_sponsorship() {
        let admin: T::AccountId = account("admin", 0, 0);
        let creator: T::AccountId = account("creator", 0, 0);
        let sponsor: T::AccountId = account("sponsor", 0, 0);
        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));
        let region_id = create_a_new_region::<T>(admin.clone());
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            creator.clone(),
            Role::ModuleCreator
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            sponsor.clone(),
            Role::ModuleSponsor
        ));

        let deposit = T::BookingDeposit::get() * 100u32.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&creator, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&sponsor, deposit));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            T::AcceptedAssets::get()[0],
            &sponsor,
            sponsor_mint_amount::<T>()
        ));

        let metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        let module_amount: u32 = 100u32;
        assert_ok!(RealXEducation::<T>::create_module(
            RawOrigin::Signed(creator.clone()).into(),
            region_id,
            module_amount,
            metadata
        ));

        assert_ok!(RealXEducation::<T>::sponsor_module(
            RawOrigin::Signed(sponsor.clone()).into(),
            0,
            10,
            T::AcceptedAssets::get()[0]
        ));

        assert!(SponsoredModules::<T>::get(0, 0).is_some());

        let expiry = System::<T>::block_number() + T::SponsorshipWindow::get() + 1u32.into();
        run_to_block::<T>(expiry);

        #[extrinsic_call]
        reclaim_unused_sponsorship(RawOrigin::Signed(sponsor.clone()), 0, 0, 10);

        let updated_module = ModuleInfo::<T>::get(0).unwrap();
        assert_eq!(updated_module.sponsor_allocation, 100);
        assert_eq!(updated_module.school_allocation, 0);
        assert!(SponsoredModules::<T>::get(0, 0).is_none());
    }

    #[benchmark]
    fn cancel_booking() {
        let admin: T::AccountId = account("admin", 0, 0);
        let creator: T::AccountId = account("creator", 0, 0);
        let sponsor: T::AccountId = account("sponsor", 0, 0);
        let school: T::AccountId = account("school", 0, 0);
        let university_student: T::AccountId = account("university_student", 0, 0);
        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));
        let region_id = create_a_new_region::<T>(admin.clone());
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            creator.clone(),
            Role::ModuleCreator
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            sponsor.clone(),
            Role::ModuleSponsor
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            school.clone(),
            Role::ModuleBooker
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            university_student.clone(),
            Role::ModuleDeliverer
        ));

        let deposit = T::BookingDeposit::get() * 100u32.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&creator, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&sponsor, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&school, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&university_student, deposit));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            T::AcceptedAssets::get()[0],
            &sponsor,
            sponsor_mint_amount::<T>()
        ));

        assert_ok!(RealXEducation::<T>::register_module_deliverer(
            RawOrigin::Signed(university_student.clone()).into(),
        ));

        let metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        let module_amount: u32 = 100u32;
        assert_ok!(RealXEducation::<T>::create_module(
            RawOrigin::Signed(creator.clone()).into(),
            region_id,
            module_amount,
            metadata
        ));

        assert_ok!(RealXEducation::<T>::sponsor_module(
            RawOrigin::Signed(sponsor.clone()).into(),
            0,
            10,
            T::AcceptedAssets::get()[0]
        ));

        let booking_metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                43u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        assert_ok!(RealXEducation::<T>::book_module(
            RawOrigin::Signed(school.clone()).into(),
            0,
            0,
            booking_metadata
        ));
        assert_ok!(RealXEducation::<T>::claim_booking(
            RawOrigin::Signed(university_student.clone()).into(),
            0,
            0
        ));

        assert_eq!(Bookings::<T>::get(0, 0).unwrap().lecturer, Some(university_student));

        #[extrinsic_call]
        cancel_booking(RawOrigin::Signed(school), 0, 0);

        assert!(Bookings::<T>::get(0, 0).is_none());
        let updated_module = ModuleInfo::<T>::get(0).unwrap();
        assert_eq!(updated_module.school_allocation, 10);
        assert_eq!(updated_module.university_student_allocation, 0);
        assert_eq!(SponsoredModules::<T>::get(0, 0).unwrap().amount, 10);
    }

    #[benchmark]
    fn clear_old_cancellations() {
        let admin: T::AccountId = account("admin", 0, 0);
        let creator: T::AccountId = account("creator", 0, 0);
        let sponsor: T::AccountId = account("sponsor", 0, 0);
        let school: T::AccountId = account("school", 0, 0);
        let university_student: T::AccountId = account("university_student", 0, 0);
        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));
        let region_id = create_a_new_region::<T>(admin.clone());
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            creator.clone(),
            Role::ModuleCreator
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            sponsor.clone(),
            Role::ModuleSponsor
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            school.clone(),
            Role::ModuleBooker
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            university_student.clone(),
            Role::ModuleDeliverer
        ));

        let deposit = T::BookingDeposit::get() * 100u32.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&creator, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&sponsor, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&school, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&university_student, deposit));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            T::AcceptedAssets::get()[0],
            &sponsor,
            sponsor_mint_amount::<T>()
        ));

        assert_ok!(RealXEducation::<T>::register_module_deliverer(
            RawOrigin::Signed(university_student.clone()).into(),
        ));

        let metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        let module_amount: u32 = 100u32;
        assert_ok!(RealXEducation::<T>::create_module(
            RawOrigin::Signed(creator.clone()).into(),
            region_id,
            module_amount,
            metadata
        ));

        assert_ok!(RealXEducation::<T>::sponsor_module(
            RawOrigin::Signed(sponsor.clone()).into(),
            0,
            10,
            T::AcceptedAssets::get()[0]
        ));

        for i in 0..<T as pallet::Config>::MaxCleanupPerCall::get() {
            let booking_metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
                BoundedVec::truncate_from(vec![
                    43u8;
                    <T as pallet::Config>::StringLimit::get() as usize
                ]);
            assert_ok!(RealXEducation::<T>::book_module(
                RawOrigin::Signed(school.clone()).into(),
                0,
                0,
                booking_metadata
            ));
            assert_ok!(RealXEducation::<T>::cancel_booking(
                RawOrigin::Signed(school.clone()).into(),
                0,
                i.into()
            ));
        }

        let current_block = System::<T>::block_number();

        assert_eq!(
            BookingCancellationCounter::<T>::get(&school),
            <T as pallet::Config>::MaxCleanupPerCall::get()
        );
        assert!(SchoolCancellations::<T>::contains_key(&school, (current_block, 0)));
        assert!(SchoolCancellations::<T>::contains_key(&school, (current_block, 49)));

        let block_number = current_block + T::CancellationWindow::get() + 1u32.into();
        run_to_block::<T>(block_number);

        #[extrinsic_call]
        clear_old_cancellations(RawOrigin::Signed(school.clone()));

        assert_eq!(BookingCancellationCounter::<T>::get(&school), 0);
        assert!(!SchoolCancellations::<T>::contains_key(&school, (current_block, 0)));
        assert!(!SchoolCancellations::<T>::contains_key(&school, (current_block, 49)));
    }

    #[benchmark]
    fn cancel_claim() {
        let admin: T::AccountId = account("admin", 0, 0);
        let creator: T::AccountId = account("creator", 0, 0);
        let sponsor: T::AccountId = account("sponsor", 0, 0);
        let school: T::AccountId = account("school", 0, 0);
        let university_student: T::AccountId = account("university_student", 0, 0);
        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));
        let region_id = create_a_new_region::<T>(admin.clone());
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            creator.clone(),
            Role::ModuleCreator
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            sponsor.clone(),
            Role::ModuleSponsor
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            school.clone(),
            Role::ModuleBooker
        ));
        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            university_student.clone(),
            Role::ModuleDeliverer
        ));

        let deposit = T::BookingDeposit::get() * 100u32.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&creator, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&sponsor, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&school, deposit));
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&university_student, deposit));
        assert_ok!(<T as pallet::Config>::ForeignCurrency::mint_into(
            T::AcceptedAssets::get()[0],
            &sponsor,
            sponsor_mint_amount::<T>()
        ));

        assert_ok!(RealXEducation::<T>::register_module_deliverer(
            RawOrigin::Signed(university_student.clone()).into(),
        ));

        let metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                42u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        let module_amount: u32 = 100u32;
        assert_ok!(RealXEducation::<T>::create_module(
            RawOrigin::Signed(creator.clone()).into(),
            region_id,
            module_amount,
            metadata
        ));

        assert_ok!(RealXEducation::<T>::sponsor_module(
            RawOrigin::Signed(sponsor.clone()).into(),
            0,
            10,
            T::AcceptedAssets::get()[0]
        ));

        let booking_metadata: BoundedVec<u8, <T as pallet::Config>::StringLimit> =
            BoundedVec::truncate_from(vec![
                43u8;
                <T as pallet::Config>::StringLimit::get() as usize
            ]);

        assert_ok!(RealXEducation::<T>::book_module(
            RawOrigin::Signed(school.clone()).into(),
            0,
            0,
            booking_metadata
        ));
        assert_ok!(RealXEducation::<T>::claim_booking(
            RawOrigin::Signed(university_student.clone()).into(),
            0,
            0
        ));

        assert_eq!(Bookings::<T>::get(0, 0).unwrap().lecturer, Some(university_student.clone()));

        #[extrinsic_call]
        cancel_claim(RawOrigin::Signed(university_student.clone()), 0, 0);

        assert_eq!(Bookings::<T>::get(0, 0).unwrap().lecturer, None);
        assert_eq!(ModuleDeliverer::<T>::get(&university_student).unwrap().active_claims, 0);
    }

    #[benchmark]
    fn register_module_deliverer() {
        let admin: T::AccountId = account("admin", 0, 0);
        let university_student: T::AccountId = account("university_student", 0, 0);
        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));

        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            university_student.clone(),
            Role::ModuleDeliverer
        ));

        let deposit = T::ModuleDelivererDeposit::get() * 10u32.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&university_student, deposit));

        assert!(ModuleDeliverer::<T>::get(&university_student).is_none());

        #[extrinsic_call]
        register_module_deliverer(RawOrigin::Signed(university_student.clone()));

        let module_deliverer = ModuleDeliverer::<T>::get(&university_student).unwrap();
        assert_eq!(module_deliverer.deposit, T::ModuleDelivererDeposit::get());
        assert_eq!(module_deliverer.active_claims, 0);
        assert_eq!(module_deliverer.active_strikes, 0);
    }

    #[benchmark]
    fn unregister_module_deliverer() {
        let admin: T::AccountId = account("admin", 0, 0);
        let university_student: T::AccountId = account("university_student", 0, 0);
        assert_ok!(Whitelist::<T>::add_admin(RawOrigin::Root.into(), admin.clone()));

        assert_ok!(Whitelist::<T>::assign_role(
            RawOrigin::Signed(admin.clone()).into(),
            university_student.clone(),
            Role::ModuleDeliverer
        ));

        let deposit = T::ModuleDelivererDeposit::get() * 10u32.into();
        assert_ok!(<T as pallet::Config>::NativeCurrency::mint_into(&university_student, deposit));

        assert_ok!(RealXEducation::<T>::register_module_deliverer(
            RawOrigin::Signed(university_student.clone()).into(),
        ));

        assert!(ModuleDeliverer::<T>::get(&university_student).is_some());

        #[extrinsic_call]
        unregister_module_deliverer(RawOrigin::Signed(university_student.clone()));

        assert!(ModuleDeliverer::<T>::get(&university_student).is_none());
    }

    impl_benchmark_test_suite!(RealXEducation, crate::mock::new_test_ext(), crate::mock::Test);
}
