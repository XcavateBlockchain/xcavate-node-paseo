use cumulus_primitives_core::ParaId;
use xcavate_runtime::{
    constants::currency::{EXISTENTIAL_DEPOSIT, CENTS}, AccountId, AuraId, Signature, Balance
};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec =
    sc_service::GenericChainSpec<xcavate_runtime::RuntimeGenesisConfig, Extensions>;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
    /// The relay chain of the Parachain.
    pub relay_chain: String,
    /// The id of the Parachain.
    pub para_id: u32,
}

impl Extensions {
    /// Try to get the extension from the given `ChainSpec`.
    pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
        sc_chain_spec::get_extension(chain_spec.extensions())
    }
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate collator keys from seed.
///
/// This function's return type must always match the session keys of the chain
/// in tuple format.
pub fn get_collator_keys_from_seed(seed: &str) -> AuraId {
    get_from_seed::<AuraId>(seed)
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we
/// have just one key).
pub fn template_session_keys(keys: AuraId) -> xcavate_runtime::SessionKeys {
    xcavate_runtime::SessionKeys { aura: keys }
}

pub fn development_config() -> ChainSpec {
    let mut properties = sc_chain_spec::Properties::new();
    properties.insert("tokenSymbol".into(), "XCAV".into());
    properties.insert("tokenDecimals".into(), 12.into());
    properties.insert("ss58Format".into(), 42.into());
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
            (
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                get_collator_keys_from_seed("Alice"),
            ),
            (
                get_account_id_from_seed::<sr25519::Public>("Bob"),
                get_collator_keys_from_seed("Bob"),
            ),
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
    properties.insert("ss58Format".into(), 42.into());

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
            (
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                get_collator_keys_from_seed("Alice"),
            ),
            (
                get_account_id_from_seed::<sr25519::Public>("Bob"),
                get_collator_keys_from_seed("Bob"),
            ),
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
    pub const ENDOWMENT: Balance = 100 * CENTS;

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
        "treasury": {},
        "polkadotXcm": {
            "safeXcmVersion": Some(SAFE_XCM_VERSION),
        },
        "sudo": { "key": Some(root) }
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
