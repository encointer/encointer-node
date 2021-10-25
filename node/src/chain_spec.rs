use encointer_node_notee_runtime::{
	AccountId, AuraConfig, BalanceType, BalancesConfig, CeremonyPhaseType, Demurrage,
	EncointerBalancesConfig, EncointerCeremoniesConfig, EncointerCommunitiesConfig,
	EncointerSchedulerConfig, GenesisConfig, GrandpaConfig, Signature, SudoConfig, SystemConfig,
	WASM_BINARY, TreasuryPalletId
};
use jsonrpc_core::serde_from_str;
use sc_service::{ChainType, Properties};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify, AccountIdConversion};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;
pub const TREASURY_FUNDING_PERCENT: u128 = 100;
pub const ENDOWED_FUNDING: u128 = 1 << 60;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
	(get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

///Get the account id for the treasury
pub fn treasury_account_id() -> AccountId {
	TreasuryPalletId::get().into_account()
}

fn properties() -> Option<Properties> {
	serde_from_str(
		r#"{
    "ss58Format": 42,
    "tokenDecimals": 12,
    "tokenSymbol": "ERT"
  }"#,
	)
	.ok()
}

pub fn development_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"Development",
		// ID
		"dev",
		ChainType::Development,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice")],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
					treasury_account_id(),
				],
				true,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		properties(),
		// Extensions
		None,
	))
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"Local Testnet",
		// ID
		"local_testnet",
		ChainType::Local,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
					get_account_id_from_seed::<sr25519::Public>("Dave"),
					get_account_id_from_seed::<sr25519::Public>("Eve"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
					get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
					get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
					get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
					treasury_account_id(),
				],
				true,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		properties(),
		// Extensions
		None,
	))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AuraId, GrandpaId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	_enable_println: bool,
) -> GenesisConfig {
	let treasury_funding = (endowed_accounts.len() as u128 - 1u128)* ENDOWED_FUNDING * TREASURY_FUNDING_PERCENT /100u128;
	GenesisConfig {
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: BalancesConfig {
			// Configure endowed accounts with initial balance of ENDOWED_FUNDING and allocate the treasury TREASURY_FUNDING_PERCENT of total supply .
			balances: endowed_accounts.iter().cloned().map(|k| {
				if k == treasury_account_id()
				{
					(k, treasury_funding)
				} else {
					(k, ENDOWED_FUNDING)
				}
			}).collect(),
		},
		aura: AuraConfig {
			authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
		},
		grandpa: GrandpaConfig {
			authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect(),
		},
		sudo: SudoConfig {
			// Assign network admin rights.
			key: root_key,
		},
		encointer_scheduler: EncointerSchedulerConfig {
			current_phase: CeremonyPhaseType::REGISTERING,
			current_ceremony_index: 1,
			ceremony_master: get_account_id_from_seed::<sr25519::Public>("Alice"),
			phase_durations: vec![
				(CeremonyPhaseType::REGISTERING, 57600000),
				(CeremonyPhaseType::ASSIGNING, 28800000),
				(CeremonyPhaseType::ATTESTING, 172800000),
			],
		},
		encointer_ceremonies: EncointerCeremoniesConfig {
			ceremony_reward: BalanceType::from_num(1),
			time_tolerance: 600_000,   // +-10min
			location_tolerance: 1_000, // [m]
		},
		encointer_communities: EncointerCommunitiesConfig {
			community_master: get_account_id_from_seed::<sr25519::Public>("Alice"),
		},
		encointer_balances: EncointerBalancesConfig {
			demurrage_per_block_default: Demurrage::from_bits(
				0x0000000000000000000001E3F0A8A973_i128,
			),
		},
		treasury: Default::default(),
	}
}
