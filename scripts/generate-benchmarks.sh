#!/bin/bash

pallets=(
    pallet_assets
    pallet_balances
    pallet_session
    pallet_timestamp
    pallet_message_queue
    pallet_sudo
    pallet_collator_selection  
    cumulus_pallet_xcmp_queue  
    cumulus_pallet_parachain_system
    pallet_proxy
    pallet_multisig
    pallet_xcm
    pallet_utility
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