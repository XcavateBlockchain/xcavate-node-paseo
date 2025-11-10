//! Integration tests for Xcavate's token gateway functionality
//!
//! This library provides utilities and tests for verifying cross-chain
//! asset transfers via Hyperbridge/ISMP.
//!
//! ## Test Modules
//!
//! - `mock` - Mock utilities for creating test ISMP messages and accounts
//! - `tests` - Unit tests for message structure validation and integration tests

#![cfg(test)]

pub mod mock;
pub mod tests;
