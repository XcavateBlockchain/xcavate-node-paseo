use xcavate_runtime::Balance;

pub const UNIT: Balance = 1_000_000_000_000;

// para_id & relay_chain
pub const PARACHAIN_ID: u32 = 4683;
pub const RELAY_CHAIN: &str = "paseo";

// token properties
pub const TOKEN_SYMBOL: &str = "XCAV";
pub const TOKEN_DECIMALS: u32 = 12;
pub const SS58_FORMAT: u32 = 0;

pub const ENDOWMENT: Balance = 100 * UNIT;