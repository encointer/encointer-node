use crate::{
	cli_args::EncointerArgsExtractor,
	utils::{get_chain_api, keys::get_pair_from_str},
};
use clap::ArgMatches;
use encointer_api_client_extension::{
	set_api_extrisic_params_builder, CommunitiesApi, EncointerXt, ParentchainExtrinsicSigner,
	ReputationRingsApi,
};
use log::info;
use parity_scale_codec::{Decode, Encode};
use sp_core::{
	bandersnatch as bandersnatch_core, crypto::Ss58Codec, sr25519 as sr25519_core, Pair,
};
use substrate_api_client::{ac_compose_macros::compose_extrinsic, SubmitAndWatch, XtStatus};

/// Maximum ring size matching runtime `MaxRingSize`.
const MAX_RING_SIZE: usize = 255;

/// Derive a Bandersnatch keypair from an account string (SURI: `//Alice`, mnemonic, `0x` seed).
fn get_bandersnatch_pair(account: &str) -> bandersnatch_core::Pair {
	bandersnatch_core::Pair::from_string(account, None)
		.expect("valid account string for Bandersnatch key derivation")
}

/// Build `VrfSignData` for proof-of-personhood context.
fn pop_vrf_sign_data(
	cid: &encointer_primitives::communities::CommunityIdentifier,
	ceremony_index: u32,
	level: u8,
	sub_ring: u32,
) -> bandersnatch_core::vrf::VrfSignData {
	let vrf_input_data = (b"encointer-pop", cid, ceremony_index, level, sub_ring).encode();
	bandersnatch_core::vrf::VrfSignData::new(&vrf_input_data, &[])
}

/// Register a Bandersnatch public key for an account.
pub fn register_bandersnatch_key(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();
		let key: [u8; 32] = if let Some(key_hex) = matches.value_of("key") {
			let key_bytes = hex::decode(key_hex.trim_start_matches("0x"))
				.expect("Invalid hex for Bandersnatch key");
			assert!(key_bytes.len() == 32, "Bandersnatch key must be 32 bytes");
			let mut k = [0u8; 32];
			k.copy_from_slice(&key_bytes);
			k
		} else {
			let account_str = matches.account_arg().expect("account required");
			let pair = get_bandersnatch_pair(account_str);
			let public: bandersnatch_core::Public = Pair::public(&pair);
			public.into()
		};

		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));

		info!("Registering Bandersnatch key for {}", who.public().to_ss58check());

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		let xt: EncointerXt<_> =
			compose_extrinsic!(api, "EncointerReputationRings", "register_bandersnatch_key", key)
				.unwrap();

		let result = api.submit_and_watch_extrinsic_until(xt, XtStatus::Ready).await;

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
			"EncointerReputationRings",
			"initiate_rings",
			cid,
			ceremony_index
		)
		.unwrap();

		let result = api.submit_and_watch_extrinsic_until(xt, XtStatus::Ready).await;

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
			compose_extrinsic!(api, "EncointerReputationRings", "continue_ring_computation")
				.unwrap();

		let result = api.submit_and_watch_extrinsic_until(xt, XtStatus::Ready).await;

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
			match api.get_sub_ring_count(cid, ceremony_index, level, maybe_at).await {
				Ok(count) if count > 0 => {
					let mut total_members = 0u32;
					for sub_idx in 0..count {
						match api
							.get_ring_members(cid, ceremony_index, level, sub_idx, maybe_at)
							.await
						{
							Ok(Some(members)) => {
								total_members += members.len() as u32;
								if count > 1 {
									println!(
										"  Level {}/5 sub-ring {}/{}: {} members",
										level,
										sub_idx + 1,
										count,
										members.len()
									);
								}
								for key in members.iter() {
									println!("    0x{}", hex::encode(key));
								}
							},
							Ok(None) => {},
							Err(e) => {
								println!(
									"  Level {}/5 sub-ring {}: error: {:?}",
									level, sub_idx, e
								);
							},
						}
					}
					println!(
						"  Level {}/5: {} members ({} sub-rings)",
						level, total_members, count
					);
				},
				Ok(_) => {
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

/// Produce a ring-VRF proof of personhood.
pub fn prove_personhood(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let account_str = matches.account_arg().expect("account required");
		let api = get_chain_api(matches).await;

		let cid = api.verify_cid(matches.cid_arg().expect("please supply --cid"), None).await;
		let ceremony_index: u32 = matches
			.value_of("ceremony-index")
			.expect("--ceremony-index required")
			.parse()
			.expect("ceremony-index must be a u32");
		let level: u8 = matches.value_of("level").unwrap_or("1").parse().expect("level must be u8");
		let sub_ring: u32 = matches
			.value_of("sub-ring")
			.unwrap_or("0")
			.parse()
			.expect("sub-ring must be u32");

		// Fetch ring members
		let members = api
			.get_ring_members(cid, ceremony_index, level, sub_ring, None)
			.await
			.expect("failed to query ring members")
			.expect("no ring found for given parameters");

		// Derive Bandersnatch keypair and find our index in the ring
		let pair = get_bandersnatch_pair(account_str);
		let public: bandersnatch_core::Public = Pair::public(&pair);
		let public_bytes: [u8; 32] = public.into();

		let prover_idx = members
			.iter()
			.position(|k| *k == public_bytes)
			.expect("account's Bandersnatch key not found in ring");

		let ring_keys: Vec<bandersnatch_core::Public> =
			members.iter().map(|k| bandersnatch_core::Public::from(*k)).collect();

		// Create ring context and prover
		let ring_ctx = bandersnatch_core::ring_vrf::RingContext::<MAX_RING_SIZE>::new_testing();
		let prover = ring_ctx.prover(&ring_keys, prover_idx);

		// Sign
		let data = pop_vrf_sign_data(&cid, ceremony_index, level, sub_ring);
		let signature = pair.ring_vrf_sign(&data, &prover);

		// Output
		let pseudonym = signature.pre_output.make_bytes();
		let sig_bytes = signature.encode();
		println!("pseudonym: 0x{}", hex::encode(pseudonym));
		println!("signature: 0x{}", hex::encode(&sig_bytes));

		Ok(())
	})
	.into()
}

/// Verify a ring-VRF proof of personhood.
pub fn verify_personhood(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let sig_hex = matches.value_of("signature").expect("--signature required");
		let sig_bytes =
			hex::decode(sig_hex.trim_start_matches("0x")).expect("Invalid hex for signature");
		let signature =
			bandersnatch_core::ring_vrf::RingVrfSignature::decode(&mut sig_bytes.as_slice())
				.expect("Failed to decode RingVrfSignature");

		let api = get_chain_api(matches).await;

		let cid = api.verify_cid(matches.cid_arg().expect("please supply --cid"), None).await;
		let ceremony_index: u32 = matches
			.value_of("ceremony-index")
			.expect("--ceremony-index required")
			.parse()
			.expect("ceremony-index must be a u32");
		let level: u8 = matches.value_of("level").unwrap_or("1").parse().expect("level must be u8");
		let sub_ring: u32 = matches
			.value_of("sub-ring")
			.unwrap_or("0")
			.parse()
			.expect("sub-ring must be u32");

		// Fetch ring members
		let members = api
			.get_ring_members(cid, ceremony_index, level, sub_ring, None)
			.await
			.expect("failed to query ring members")
			.expect("no ring found for given parameters");

		let ring_keys: Vec<bandersnatch_core::Public> =
			members.iter().map(|k| bandersnatch_core::Public::from(*k)).collect();

		// Create ring context and verifier
		let ring_ctx = bandersnatch_core::ring_vrf::RingContext::<MAX_RING_SIZE>::new_testing();
		let verifier = ring_ctx.verifier(&ring_keys);

		// Verify
		let data = pop_vrf_sign_data(&cid, ceremony_index, level, sub_ring);
		if signature.ring_vrf_verify(&data, &verifier) {
			let pseudonym = signature.pre_output.make_bytes();
			println!("VALID");
			println!("pseudonym: 0x{}", hex::encode(pseudonym));
		} else {
			println!("INVALID");
			std::process::exit(1);
		}

		Ok(())
	})
	.into()
}
