use crate::{
	cli_args::EncointerArgsExtractor,
	utils::{get_chain_api, keys::get_pair_from_str},
};
use clap::ArgMatches;
use encointer_api_client_extension::{
	set_api_extrisic_params_builder, CommunitiesApi, EncointerXt, ParentchainExtrinsicSigner,
	ReputationRingApi,
};
use log::info;
use sp_core::{crypto::Ss58Codec, sr25519 as sr25519_core, Pair};
use substrate_api_client::{ac_compose_macros::compose_extrinsic, SubmitAndWatch, XtStatus};

/// Register a Bandersnatch public key for an account.
pub fn register_bandersnatch_key(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();
		let key_hex = matches.value_of("key").expect("--key required");
		let key_bytes = hex::decode(key_hex.trim_start_matches("0x"))
			.expect("Invalid hex for Bandersnatch key");
		assert!(key_bytes.len() == 32, "Bandersnatch key must be 32 bytes");
		let mut key = [0u8; 32];
		key.copy_from_slice(&key_bytes);

		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));

		info!("Registering Bandersnatch key for {}", who.public().to_ss58check());

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		let xt: EncointerXt<_> =
			compose_extrinsic!(api, "EncointerReputationRing", "register_bandersnatch_key", key)
				.unwrap();

		let result = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await;

		match result {
			Ok(_report) => {
				println!("Bandersnatch key registered for {}", who.public().to_ss58check());
				println!("Key: 0x{}", hex::encode(key));
			},
			Err(e) => {
				println!("Failed to register Bandersnatch key: {:?}", e);
			},
		}

		Ok(())
	})
	.into()
}

/// Initiate ring computation for a community at a ceremony index.
pub fn initiate_rings(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let signer = matches.account_arg().map(get_pair_from_str).unwrap();

		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(signer)));

		let cid = api.verify_cid(matches.cid_arg().expect("please supply --cid"), None).await;
		let ceremony_index: u32 = matches
			.value_of("ceremony-index")
			.expect("--ceremony-index required")
			.parse()
			.expect("ceremony-index must be a u32");

		info!("Initiating rings for community {} at ceremony index {}", cid, ceremony_index);

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		let xt: EncointerXt<_> = compose_extrinsic!(
			api,
			"EncointerReputationRing",
			"initiate_rings",
			cid,
			ceremony_index
		)
		.unwrap();

		let result = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await;

		match result {
			Ok(_report) => {
				println!("Ring computation initiated for {} at cindex {}", cid, ceremony_index);
			},
			Err(e) => {
				println!("Failed to initiate rings: {:?}", e);
			},
		}

		Ok(())
	})
	.into()
}

/// Continue the pending ring computation (one step).
pub fn continue_ring_computation(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let signer = matches.account_arg().map(get_pair_from_str).unwrap();

		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(signer)));

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		let xt: EncointerXt<_> =
			compose_extrinsic!(api, "EncointerReputationRing", "continue_ring_computation")
				.unwrap();

		let result = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await;

		match result {
			Ok(_report) => {
				println!("Ring computation step completed");
			},
			Err(e) => {
				println!("Failed to continue ring computation: {:?}", e);
			},
		}

		Ok(())
	})
	.into()
}

/// Query and print ring members for a community and ceremony index.
pub fn get_rings(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let maybe_at = matches.at_block_arg();

		let cid = api.verify_cid(matches.cid_arg().expect("please supply --cid"), None).await;
		let ceremony_index: u32 = matches
			.value_of("ceremony-index")
			.expect("--ceremony-index required")
			.parse()
			.expect("ceremony-index must be a u32");

		println!("Rings for community {} at ceremony index {}:", cid, ceremony_index);

		for level in 1..=5u8 {
			match api.get_ring_members(cid, ceremony_index, level, maybe_at).await {
				Ok(Some(members)) => {
					println!("  Level {}/5: {} members", level, members.len());
					for key in members.iter() {
						println!("    0x{}", hex::encode(key));
					}
				},
				Ok(None) => {
					println!("  Level {}/5: no ring", level);
				},
				Err(e) => {
					println!("  Level {}/5: error querying: {:?}", level, e);
				},
			}
		}

		Ok(())
	})
	.into()
}
