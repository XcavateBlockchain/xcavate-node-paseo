#!/bin/bash

pallets=(
    #pallet_assets
    #pallet_balances
    #pallet_session
    #pallet_timestamp
    #pallet_message_queue
    #pallet_sudo
    #pallet_collator_selection  
    #cumulus_pallet_xcmp_queue  
    #cumulus_pallet_parachain_system
    #pallet_proxy
    #pallet_multisig
    #pallet_xcm
    #pallet_utility
    #cumulus_pallet_weight_reclaim
    #pallet_transaction_payment
    #pallet_vesting
    #pallet_nfts
    #pallet_nft_fractionalization
    #pallet_xcavate_whitelist
    #pallet_education_regions
    #pallet_real_x_education
    #pallet_regions
    pallet_marketplace
    pallet_property_management
    pallet_property_governance
    #pallet_bucket
    #attestation
    #ctype
    #delegation
    #did
    #pallet-public_credentials
    #pallet_did_lookup
    #pallet_faucet
)

# Generate weights
for pallet_name in "${pallets[@]}"; do
    ./target/release/xcavate-node benchmark pallet \
        --pallet $pallet_name \
        --extrinsic "*" \
        --steps 50 \
        --repeat 20 \
        --output ./runtime/src/weights/$pallet_name.rs
done