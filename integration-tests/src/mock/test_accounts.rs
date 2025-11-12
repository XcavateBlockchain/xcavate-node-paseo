//! Test account utilities
//!
//! Provides predefined Ethereum addresses and Xcavate accounts for testing.

use hex_literal::hex;
use sp_core::crypto::AccountId32;

/// Mock Ethereum address (20 bytes)
pub const ALICE_ETH_ADDRESS: [u8; 20] = hex!("70997970C51812dc3A010C7d01b50e0d17dc79C8");

/// Mock Ethereum address for Bob
pub const BOB_ETH_ADDRESS: [u8; 20] = hex!("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC");

/// Mock Ethereum address for Charlie
pub const CHARLIE_ETH_ADDRESS: [u8; 20] = hex!("90F79bf6EB2c4f870365E785982E1f101E93b906");

/// Mock Xcavate account for Alice
pub fn alice_account() -> AccountId32 {
    AccountId32::new([1u8; 32])
}

/// Mock Xcavate account for Bob
pub fn bob_account() -> AccountId32 {
    AccountId32::new([2u8; 32])
}

/// Mock Xcavate account for Charlie
pub fn charlie_account() -> AccountId32 {
    AccountId32::new([3u8; 32])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ethereum_addresses_are_20_bytes_long() {
        assert_eq!(ALICE_ETH_ADDRESS.len(), 20);
        assert_eq!(BOB_ETH_ADDRESS.len(), 20);
        assert_eq!(CHARLIE_ETH_ADDRESS.len(), 20);
    }

    #[test]
    fn accounts_do_not_repeat() {
        let alice = alice_account();
        let bob = bob_account();
        let charlie = charlie_account();

        assert_ne!(alice, bob);
        assert_ne!(bob, charlie);
        assert_ne!(alice, charlie);
    }
}
