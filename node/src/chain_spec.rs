use cumulus_primitives_core::ParaId;
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::crypto::Ss58Codec;
use sp_keyring::Sr25519Keyring;
use xcavate_runtime::{
    constants::currency::{EXISTENTIAL_DEPOSIT, XCAV},
    AccountId, AuraId, Balance,
};

use crate::constant::xcavate;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<Extensions>;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
pub struct Extensions {
    /// The relay chain of the Parachain.
    #[serde(alias = "relayChain", alias = "RelayChain")]
    pub relay_chain: String,
    /// The id of the Parachain.
    #[serde(alias = "paraId", alias = "ParaId")]
    pub para_id: u32,
}

impl Extensions {
    /// Try to get the extension from the given `ChainSpec`.
    pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
        sc_chain_spec::get_extension(chain_spec.extensions())
    }
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we
/// have just one key).
pub fn template_session_keys(keys: AuraId) -> xcavate_runtime::SessionKeys {
    xcavate_runtime::SessionKeys { aura: keys }
}

pub fn polkadot_live_xcavate_config() -> ChainSpec {
    // Give your base currency a unit name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), xcavate::TOKEN_SYMBOL.into());
    properties.insert("tokenDecimals".into(), xcavate::TOKEN_DECIMALS.into());
    properties.insert("ss58Format".into(), xcavate::SS58_FORMAT.into());
    // This is very important for us, it lets us track the usage of our templates, and have no downside for the node/runtime. Please do not remove :)
    properties.insert("basedOn".into(), "OpenZeppelin Generic Template".into());

    // collators1 - ganesh
    let collator_0_account_id: AccountId =
        AccountId::from_ss58check("5CyBrku1V4d2WF965k1DeqvpFc3MmyuyhvtkgFYqJvpJf89S").unwrap();
    let collator_0_aura_id: AuraId =
        AuraId::from_ss58check("5CyBrku1V4d2WF965k1DeqvpFc3MmyuyhvtkgFYqJvpJf89S").unwrap();

    ChainSpec::builder(
        xcavate_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
        Extensions {
            relay_chain: xcavate::RELAY_POLKADOT_CHAIN.into(),
            // You MUST set this to the correct network!
            para_id: xcavate::PARACHAIN_POLKADOT_ID,
        },
    )
    .with_name("Xcavate Polkadot")
    .with_id("xcavate polkadot")
    .with_chain_type(ChainType::Live)
    .with_genesis_config_patch(live_genesis(
        // initial collators.
        vec![
            // XCAVATE COLLATOR 0
            (collator_0_account_id, collator_0_aura_id),
        ],
        get_endowed_accounts(),
        get_root_account(),
        xcavate::PARACHAIN_POLKADOT_ID.into(),
    ))
    .with_protocol_id("xcavate-polkadot-chain")
    .with_properties(properties)
    .build()
}

pub fn live_xcavate_config() -> ChainSpec {
    // Give your base currency a unit name and decimal places
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), xcavate::TOKEN_SYMBOL.into());
    properties.insert("tokenDecimals".into(), xcavate::TOKEN_DECIMALS.into());
    properties.insert("ss58Format".into(), xcavate::SS58_FORMAT.into());
    // This is very important for us, it lets us track the usage of our templates, and have no downside for the node/runtime. Please do not remove :)
    properties.insert("basedOn".into(), "OpenZeppelin Generic Template".into());

    // collators1 - ganesh
    let collator_0_account_id: AccountId =
        AccountId::from_ss58check("5CyBrku1V4d2WF965k1DeqvpFc3MmyuyhvtkgFYqJvpJf89S").unwrap();
    let collator_0_aura_id: AuraId =
        AuraId::from_ss58check("5CyBrku1V4d2WF965k1DeqvpFc3MmyuyhvtkgFYqJvpJf89S").unwrap();

    ChainSpec::builder(
        xcavate_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
        Extensions {
            relay_chain: xcavate::RELAY_PASEO_CHAIN.into(),
            // You MUST set this to the correct network!
            para_id: xcavate::PARACHAIN_PASEO_ID,
        },
    )
    .with_name("Xcavate")
    .with_id("xcavate")
    .with_chain_type(ChainType::Live)
    .with_genesis_config_patch(live_genesis(
        // initial collators.
        vec![
            // XCAVATE COLLATOR 0
            (collator_0_account_id, collator_0_aura_id),
        ],
        get_endowed_accounts(),
        get_root_account(),
        xcavate::PARACHAIN_PASEO_ID.into(),
    ))
    .with_protocol_id("xcavate-chain")
    .with_properties(properties)
    .build()
}

pub fn development_config() -> ChainSpec {
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "XCAV".into());
    properties.insert("tokenDecimals".into(), 12.into());
    properties.insert("ss58Format".into(), 0.into());
    // This is very important for us, it lets us track the usage of our templates, and have no downside for the node/runtime. Please do not remove :)
    properties.insert("basedOn".into(), "OpenZeppelin Generic Template".into());

    ChainSpec::builder(
        xcavate_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
        Extensions {
            relay_chain: "rococo-local".into(),
            // You MUST set this to the correct network!
            para_id: 1000,
        },
    )
    .with_name("Development")
    .with_id("dev")
    .with_chain_type(ChainType::Development)
    .with_genesis_config_patch(testnet_genesis(
        // initial collators.
        vec![
            (Sr25519Keyring::Alice.to_account_id(), Sr25519Keyring::Alice.public().into()),
            (Sr25519Keyring::Bob.to_account_id(), Sr25519Keyring::Bob.public().into()),
        ],
        get_endowed_accounts(),
        get_root_account(),
        1000.into(),
    ))
    .build()
}

pub fn local_testnet_config() -> ChainSpec {
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "XCAV".into());
    properties.insert("tokenDecimals".into(), 12.into());
    properties.insert("ss58Format".into(), 0.into());

    #[allow(deprecated)]
    ChainSpec::builder(
        xcavate_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
        Extensions {
            relay_chain: "rococo-local".into(),
            // You MUST set this to the correct network!
            para_id: 1000,
        },
    )
    .with_name("Local Testnet")
    .with_id("local_testnet")
    .with_chain_type(ChainType::Local)
    .with_genesis_config_patch(testnet_genesis(
        // initial collators.
        vec![
            (Sr25519Keyring::Alice.to_account_id(), Sr25519Keyring::Alice.public().into()),
            (Sr25519Keyring::Bob.to_account_id(), Sr25519Keyring::Bob.public().into()),
        ],
        get_endowed_accounts(),
        get_root_account(),
        1000.into(),
    ))
    .with_protocol_id("template-local")
    .with_properties(properties)
    .build()
}

fn testnet_genesis(
    invulnerables: Vec<(AccountId, AuraId)>,
    endowed_accounts: Vec<AccountId>,
    root: AccountId,
    id: ParaId,
) -> serde_json::Value {
    pub const ENDOWMENT: Balance = 100 * XCAV;

    serde_json::json!({
        "balances": {
            "balances": endowed_accounts.iter().cloned().map(|k| (k, ENDOWMENT)).collect::<Vec<_>>(),
        },
        "parachainInfo": {
            "parachainId": id,
        },
        "collatorSelection": {
            "invulnerables": invulnerables.iter().cloned().map(|(acc, _)| acc).collect::<Vec<_>>(),
            "candidacyBond": EXISTENTIAL_DEPOSIT * 16,
        },
        "session": {
            "keys": invulnerables
                .into_iter()
                .map(|(acc, aura)| {
                    (
                        acc.clone(),                 // account id
                        acc,                         // validator id
                        template_session_keys(aura), // session keys
                    )
                })
            .collect::<Vec<_>>(),
        },
        "polkadotXcm": {
            "safeXcmVersion": Some(SAFE_XCM_VERSION),
        },
        "sudo": { "key": Some(root.clone()) },
        "assets" : {
            "assets": vec![
                (10, root.clone(), true, 1),
                (1337, root.clone(), true, 1),
                (1984, root.clone(), true, 1),
            ],
            "metadata": vec![
                (10, "tGBP".as_bytes(), "tGBP".as_bytes(), 18),
                (1337, "USDC".as_bytes(), "USDC".as_bytes(), 6),
                (1984, "USDT".as_bytes(), "USDT".as_bytes(), 6),
            ],
            "accounts": endowed_accounts
                .iter()
                .cloned()
                .flat_map(|x| vec![
                    (10, x.clone(), 10_000_000_000_000_000_000u64),
                    (1337, x.clone(), 2_000_000_000_000u64),
                    (1984, x.clone(), 2_000_000_000_000u64),
                ])
                .collect::<Vec<_>>(),
        }
    })
}

fn live_genesis(
    invulnerables: Vec<(AccountId, AuraId)>,
    endowed_accounts: Vec<AccountId>,
    root: AccountId,
    id: ParaId,
) -> serde_json::Value {
    serde_json::json!({
        "balances": {
            "balances": endowed_accounts.iter().cloned().map(|k| (k, xcavate::ENDOWMENT)).collect::<Vec<_>>(),
        },
        "parachainInfo": {
            "parachainId": id,
        },
        "collatorSelection": {
            "invulnerables": invulnerables.iter().cloned().map(|(acc, _)| acc).collect::<Vec<_>>(),
            "candidacyBond": EXISTENTIAL_DEPOSIT * 16,
        },
        "session": {
            "keys": invulnerables
                .into_iter()
                .map(|(acc, aura)| {
                    (
                        acc.clone(),                 // account id
                        acc,                         // validator id
                        template_session_keys(aura), // session keys
                    )
                })
            .collect::<Vec<_>>(),
        },
        "polkadotXcm": {
            "safeXcmVersion": Some(SAFE_XCM_VERSION),
        },
        "sudo": { "key": Some(root.clone()) },
        "assets" : {
            "assets": vec![
                (10, root.clone(), true, 1),
                (1337, root.clone(), true, 1),
                (1984, root.clone(), true, 1),
            ],
            "metadata": vec![
                (10, "tGBP".as_bytes(), "tGBP".as_bytes(), 18),
                (1337, "USDC".as_bytes(), "USDC".as_bytes(), 6),
                (1984, "USDT".as_bytes(), "USDT".as_bytes(), 6),
            ],
            "accounts": endowed_accounts
                .iter()
                .cloned()
                .flat_map(|x| vec![
                    (10, x.clone(), 10_000_000_000_000_000_000u64),
                    (1337, x.clone(), 2_000_000_000_000u64),
                    (1984, x.clone(), 2_000_000_000_000u64),
                ])
                .collect::<Vec<_>>(),
        },
    })
}

pub fn get_root_account() -> AccountId {
    let json_data = &include_bytes!("../../seed/accounts.json")[..];
    let additional_accounts_with_balance: Vec<AccountId> =
        serde_json::from_slice(json_data).unwrap_or_default();

    additional_accounts_with_balance[0].clone()
}

pub fn get_endowed_accounts() -> Vec<AccountId> {
    let json_data = &include_bytes!("../../seed/accounts.json")[..];
    let additional_accounts_with_balance: Vec<AccountId> =
        serde_json::from_slice(json_data).unwrap_or_default();

    additional_accounts_with_balance
}
