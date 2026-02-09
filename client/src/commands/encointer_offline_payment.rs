use crate::{
	cli_args::EncointerArgsExtractor,
	utils::{
		contains_sudo_pallet, get_chain_api,
		keys::{get_accountid_from_str, get_pair_from_str},
		print_raw_call, send_and_wait_for_in_block, sudo_call, xt, OpaqueCall,
	},
};
use clap::ArgMatches;
use encointer_api_client_extension::{
	set_api_extrisic_params_builder, CommunitiesApi, EncointerXt, ParentchainExtrinsicSigner,
};
use encointer_primitives::balances::BalanceType;
use frame_support::BoundedVec;
use log::info;
use pallet_encointer_offline_payment::{
	circuit::{compute_commitment, poseidon_config},
	derive_zk_secret,
	prover::{
		bytes32_to_field, field_to_bytes32, generate_proof, proof_to_bytes, TrustedSetup,
		TEST_SETUP_SEED,
	},
};
use parity_scale_codec::Encode;
use sp_core::{crypto::Ss58Codec, sr25519 as sr25519_core, Pair};
use sp_keyring::Sr25519Keyring as AccountKeyring;
use substrate_api_client::{
	ac_compose_macros::{compose_call, compose_extrinsic},
	GetStorage, SubmitAndWatch, XtStatus,
};

/// Register offline identity for an account using Poseidon commitment
pub fn register_offline_identity(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();

		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));

		// Derive zk_secret from the account's seed
		let seed_bytes = who.to_raw_vec();
		let zk_secret_bytes = derive_zk_secret(&seed_bytes);
		let zk_secret = bytes32_to_field(&zk_secret_bytes);

		// Compute Poseidon commitment
		let poseidon = poseidon_config();
		let commitment_field = compute_commitment(&poseidon, &zk_secret);
		let commitment = field_to_bytes32(&commitment_field);

		info!("Registering offline identity for {}", who.public().to_ss58check());
		info!("Commitment: 0x{}", hex::encode(commitment));

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		let xt: EncointerXt<_> = compose_extrinsic!(
			api,
			"EncointerOfflinePayment",
			"register_offline_identity",
			commitment
		)
		.unwrap();

		let result = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await;

		match result {
			Ok(report) => {
				println!("Offline identity registered successfully");
				println!("Commitment: 0x{}", hex::encode(commitment));
				for event in report.events.unwrap().iter() {
					if event.pallet_name() == "EncointerOfflinePayment" &&
						event.variant_name() == "OfflineIdentityRegistered"
					{
						println!("Event: OfflineIdentityRegistered");
					}
				}
			},
			Err(e) => {
				println!("Failed to register offline identity: {:?}", e);
			},
		};

		Ok(())
	})
	.into()
}

/// Get offline identity for an account
pub fn get_offline_identity(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let account = get_accountid_from_str(matches.account_arg().unwrap());

		let maybe_at = matches.at_block_arg();

		let commitment: Option<[u8; 32]> = api
			.get_storage_map(
				"EncointerOfflinePayment",
				"OfflineIdentities",
				account.clone(),
				maybe_at,
			)
			.await
			.unwrap();

		match commitment {
			Some(c) => {
				println!("Account: {}", account.to_ss58check());
				println!("Commitment: 0x{}", hex::encode(c));
			},
			None => {
				println!("No offline identity registered for {}", account.to_ss58check());
			},
		}

		Ok(())
	})
	.into()
}

/// Generate an offline payment with real Groth16 ZK proof
pub fn generate_offline_payment(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;

		let from = matches.signer_arg().map(get_pair_from_str).unwrap();
		let to = get_accountid_from_str(matches.value_of("to").unwrap());
		let amount_str = matches.value_of("amount").unwrap();
		let amount_f64: f64 = amount_str.parse().expect("Invalid amount");
		let amount = BalanceType::from_num(amount_f64);

		let cid = api
			.verify_cid(matches.cid_arg().expect("please supply argument --cid"), None)
			.await;

		// Derive zk_secret from sender's seed
		let seed_bytes = from.to_raw_vec();
		let zk_secret_bytes = derive_zk_secret(&seed_bytes);
		let zk_secret = bytes32_to_field(&zk_secret_bytes);

		// Generate random nonce using timestamp
		let timestamp = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos();
		let nonce = bytes32_to_field(&sp_io::hashing::blake2_256(&timestamp.to_le_bytes()));

		// Compute public inputs
		let recipient_hash_bytes = pallet_encointer_offline_payment::hash_recipient(&to.encode());
		let cid_hash_bytes = pallet_encointer_offline_payment::hash_cid(&cid);
		let amount_bytes = pallet_encointer_offline_payment::balance_to_bytes(amount);

		let recipient_hash = bytes32_to_field(&recipient_hash_bytes);
		let cid_hash = bytes32_to_field(&cid_hash_bytes);
		let amount_field = bytes32_to_field(&amount_bytes);

		// Get the test proving key
		// In production, this would be loaded from a file or generated once
		let setup = TrustedSetup::generate_with_seed(TEST_SETUP_SEED);

		// Generate the ZK proof
		eprintln!("Generating ZK proof...");
		let (proof, public_inputs) = generate_proof(
			&setup.proving_key,
			zk_secret,
			nonce,
			recipient_hash,
			amount_field,
			cid_hash,
		)
		.expect("Proof generation failed");

		let proof_bytes = proof_to_bytes(&proof);
		let commitment = field_to_bytes32(&public_inputs[0]);
		let nullifier = field_to_bytes32(&public_inputs[4]);
		let sender = get_accountid_from_str(&from.public().to_ss58check());

		// Output as JSON
		let output = serde_json::json!({
			"proof": hex::encode(&proof_bytes),
			"commitment": hex::encode(commitment),
			"sender": sender.to_ss58check(),
			"recipient": to.to_ss58check(),
			"amount": amount_str,
			"cid": cid.to_string(),
			"nullifier": hex::encode(nullifier),
		});

		println!("{}", serde_json::to_string_pretty(&output).unwrap());

		Ok(())
	})
	.into()
}

/// Submit an offline payment proof for settlement
pub fn submit_offline_payment(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let signer = matches.signer_arg().map(get_pair_from_str).unwrap();

		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(signer)));

		// Parse proof file or inline arguments
		let (proof_bytes, sender, recipient, amount, cid, nullifier) = if let Some(proof_file) =
			matches.value_of("proof-file")
		{
			let content = std::fs::read_to_string(proof_file).expect("Failed to read proof file");
			let json: serde_json::Value =
				serde_json::from_str(&content).expect("Invalid JSON in proof file");

			let proof_bytes =
				hex::decode(json["proof"].as_str().unwrap()).expect("Invalid proof hex");

			let sender = get_accountid_from_str(json["sender"].as_str().unwrap());
			let recipient = get_accountid_from_str(json["recipient"].as_str().unwrap());
			let amount =
				BalanceType::from_num(json["amount"].as_str().unwrap().parse::<f64>().unwrap());
			let cid_str = json["cid"].as_str().unwrap();
			let cid = api.verify_cid(cid_str, None).await;
			let nullifier_bytes =
				hex::decode(json["nullifier"].as_str().unwrap()).expect("Invalid nullifier hex");
			let mut nullifier = [0u8; 32];
			nullifier.copy_from_slice(&nullifier_bytes);

			(proof_bytes, sender, recipient, amount, cid, nullifier)
		} else {
			// Parse inline arguments
			let proof_hex = matches.value_of("proof").expect("proof required");
			let proof_bytes = hex::decode(proof_hex).expect("Invalid proof hex");

			let sender = get_accountid_from_str(matches.value_of("sender").unwrap());
			let recipient = get_accountid_from_str(matches.value_of("recipient").unwrap());
			let amount =
				BalanceType::from_num(matches.value_of("amount").unwrap().parse::<f64>().unwrap());
			let cid = api
				.verify_cid(matches.cid_arg().expect("please supply argument --cid"), None)
				.await;
			let nullifier_bytes =
				hex::decode(matches.value_of("nullifier").unwrap()).expect("Invalid nullifier hex");
			let mut nullifier = [0u8; 32];
			nullifier.copy_from_slice(&nullifier_bytes);

			(proof_bytes, sender, recipient, amount, cid, nullifier)
		};

		info!(
			"Submitting offline payment: {} -> {}, amount: {}, cid: {}",
			sender.to_ss58check(),
			recipient.to_ss58check(),
			amount,
			cid
		);

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		// Create the Groth16ProofBytes structure
		let bounded_proof: BoundedVec<u8, frame_support::traits::ConstU32<256>> =
			BoundedVec::try_from(proof_bytes).expect("Proof exceeds max size");

		#[derive(Clone, parity_scale_codec::Encode)]
		struct Groth16ProofBytesEncode {
			proof_bytes: BoundedVec<u8, frame_support::traits::ConstU32<256>>,
		}

		let proof = Groth16ProofBytesEncode { proof_bytes: bounded_proof };

		let xt: EncointerXt<_> = compose_extrinsic!(
			api,
			"EncointerOfflinePayment",
			"submit_offline_payment",
			proof,
			sender.clone(),
			recipient.clone(),
			amount,
			cid,
			nullifier
		)
		.unwrap();

		let result = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await;

		match result {
			Ok(report) => {
				println!("Offline payment submitted successfully");
				for event in report.events.unwrap().iter() {
					if event.pallet_name() == "EncointerOfflinePayment" {
						match event.variant_name() {
							"OfflinePaymentSettled" => {
								println!("Payment settled!");
								println!("Sender: {}", sender.to_ss58check());
								println!("Recipient: {}", recipient.to_ss58check());
								println!("Amount: {}", amount);
							},
							name => println!("Event: {}", name),
						}
					}
				}
			},
			Err(e) => {
				println!("Failed to submit offline payment: {:?}", e);
			},
		};

		Ok(())
	})
	.into()
}

/// Set the Groth16 verification key via sudo (requires --signer to be sudo key)
pub fn set_verification_key(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		// Use Alice as default signer (sudo in dev mode)
		let signer = matches.signer_arg().map_or_else(
			|| AccountKeyring::Alice.pair(),
			|signer| get_pair_from_str(signer).into(),
		);

		let mut api = get_chain_api(matches).await;
		let signer = ParentchainExtrinsicSigner::new(signer);
		api.set_signer(signer);

		// Generate the test VK if no VK provided
		let vk_bytes = if let Some(hex) = matches.value_of("vk") {
			hex::decode(hex).expect("Invalid verification key hex")
		} else {
			eprintln!("Generating test verification key with seed 0x{:X}...", TEST_SETUP_SEED);
			let setup = TrustedSetup::generate_with_seed(TEST_SETUP_SEED);
			setup.verifying_key_bytes()
		};

		info!("Setting verification key ({} bytes)", vk_bytes.len());

		// Create the inner call
		let set_vk_call = compose_call!(
			api.metadata(),
			"EncointerOfflinePayment",
			"set_verification_key",
			vk_bytes.clone()
		)
		.unwrap();

		// Wrap in sudo call
		let call = if contains_sudo_pallet(api.metadata()) {
			let sudo_call = sudo_call(api.metadata(), set_vk_call);
			info!("Submitting sudo(set_verification_key)");
			print_raw_call("sudo(set_verification_key)", &sudo_call);
			OpaqueCall::from_tuple(&sudo_call)
		} else {
			eprintln!("ERROR: Sudo pallet not found. Cannot set verification key.");
			return Ok(());
		};

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		send_and_wait_for_in_block(&api, xt(&api, call).await, tx_payment_cid_arg).await;

		println!("Verification key set successfully!");
		println!("VK size: {} bytes", vk_bytes.len());
		Ok(())
	})
	.into()
}

/// Generate and output the test verification key
pub fn generate_test_vk(_args: &str, _matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	eprintln!("Generating test verification key with seed {}...", TEST_SETUP_SEED);
	let setup = TrustedSetup::generate_with_seed(TEST_SETUP_SEED);
	let vk_bytes = setup.verifying_key_bytes();

	println!("{}", hex::encode(&vk_bytes));

	Ok(())
}
