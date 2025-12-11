# Zombienet Configuration

Zombienet is a testing framework for Substrate-based blockchains that allows you to spawn and test ephemeral networks.

## Quick Start with Pop CLI (Recommended)

The easiest way to run a local network is using [Pop CLI](https://onpop.io). Pop can launch networks directly from zombienet configuration files and handles all required binary downloads automatically.

### Install Pop CLI

**macOS (Homebrew):**
```sh
brew install r0gue-io/pop-cli/pop
```

**All platforms (Cargo):**
```sh
cargo install pop-cli
```

### Launch the Network

1. Build the xcavate node:
   ```sh
   cargo build
   ```

2. Start the local Paseo network with xcavate parachain:
   ```sh
   pop up zombienet-config/paseo-local.toml
   ```

Pop will automatically download the required relay chain binaries and bootstrap the network. Once running, you can access:

- **Relay chain (Alice):** `ws://localhost:9900`
- **Xcavate parachain:** `ws://localhost:9920`

---

## Alternative: Manual Setup

If you prefer not to use Pop CLI, you can use the zombienet scripts directly.

### Build Polkadot binaries

```sh
scripts/zombienet.sh build
```

This process can take some time. On Linux, you can alternatively download pre-built binaries:

```sh
scripts/zombienet.sh init
```

### Spawn the network

```sh
scripts/zombienet.sh devnet
```
