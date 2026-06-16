#!/bin/bash

pallets=(
    pallet_marketplace
    #pallet_property_management
    #pallet_property_governance
)

# Generate weights
for pallet_name in "${pallets[@]}"; do
    ./target/release/xcavate-node benchmark pallet \
        --pallet $pallet_name \
        --extrinsic "*" \
        --steps 50 \
        --repeat 20 \
        --output ./pallets/marketplace/src/weights.rs
done