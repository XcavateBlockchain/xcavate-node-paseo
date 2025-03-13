# script to generate chainspec, raw, wasm, genesis 
# if you want to run this file make sure para id should be change in line 15 & 18
# chmod +x build.sh
# ./build.sh

#Exit script on any error
set -e

# Build the Substrate node
# echo "🚀 Building the Substrate Xcavate node..."
# cargo build --release

# Generate the plain text chain specification for the xcavate-node
# echo "Generating the plain text chain specification for the xcavate-node..."
./target/release/xcavate-node build-spec --disable-default-bootnode > plain-parachain-chainspec.json

# Generate the raw chain specification for the modified chain specification
# echo "Generating the raw chain specification for the modified chain specification..."
./target/release/xcavate-node build-spec --chain plain-parachain-chainspec.json --disable-default-bootnode --raw > raw-parachain-chainspec.json

# Export the WebaAssembly runtime for the parachain
./target/release/xcavate-node export-genesis-wasm --chain raw-parachain-chainspec.json para-4683-wasm

# Generate a parachain genesis state
./target/release/xcavate-node export-genesis-state --chain raw-parachain-chainspec.json para-4683-genesis-state

# Move the generated chainspec, raw, wasm, genesis state to the chainspec directory
mv ./plain-parachain-chainspec.json chainspec
mv ./raw-parachain-chainspec.json chainspec
mv ./para-4683-wasm chainspec
mv ./para-4683-genesis-state chainspec