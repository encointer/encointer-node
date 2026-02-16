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
		bytes32_to_field, field_to_bytes32, generate_proof, proof_to_bytes, verify_proof,
		TrustedSetup, TEST_SETUP_SEED,
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
		let asset_hash_bytes = pallet_encointer_offline_payment::hash_cid(&cid);
		let amount_bytes = pallet_encointer_offline_payment::balance_to_bytes(amount);

		// Chain-bind the asset hash with genesis hash for cross-chain replay protection
		let genesis_hash = api.genesis_hash();
		let chain_asset_hash_bytes =
			sp_io::hashing::blake2_256(&[&asset_hash_bytes[..], genesis_hash.as_ref()].concat());

		let recipient_hash = bytes32_to_field(&recipient_hash_bytes);
		let chain_asset_hash = bytes32_to_field(&chain_asset_hash_bytes);
		let amount_field = bytes32_to_field(&amount_bytes);

		// Load proving key from file, or fall back to test key
		let pk_loaded;
		let pk_ref = if let Some(pk_path) = matches.value_of("pk-file") {
			let bytes = std::fs::read(pk_path).expect("Failed to read proving key file");
			pk_loaded = TrustedSetup::proving_key_from_bytes(&bytes)
				.expect("Failed to deserialize proving key — is it a valid PK file?");
			eprintln!("Loaded proving key from {} ({} bytes)", pk_path, bytes.len());
			&pk_loaded
		} else {
			eprintln!(
				"WARNING: Using test proving key (seed 0x{:X}). NOT for production!",
				TEST_SETUP_SEED
			);
			let setup = TrustedSetup::generate_with_seed(TEST_SETUP_SEED);
			pk_loaded = setup.proving_key;
			&pk_loaded
		};

		// Generate the ZK proof
		eprintln!("Generating ZK proof...");
		let (proof, public_inputs) = generate_proof(
			pk_ref,
			zk_secret,
			nonce,
			recipient_hash,
			amount_field,
			chain_asset_hash,
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

		// Load VK from file, hex string, or generate test key
		let vk_bytes = if let Some(file_path) = matches.value_of("vk-file") {
			let bytes = std::fs::read(file_path).expect("Failed to read VK file");
			TrustedSetup::verifying_key_from_bytes(&bytes)
				.expect("Failed to deserialize VK from file — is it a valid verifying key?");
			eprintln!("Loaded verification key from {} ({} bytes)", file_path, bytes.len());
			bytes
		} else if let Some(hex_str) = matches.value_of("vk") {
			hex::decode(hex_str).expect("Invalid verification key hex")
		} else {
			eprintln!(
				"WARNING: Using test verification key (seed 0x{:X}). NOT for production!",
				TEST_SETUP_SEED
			);
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

/// Generate a trusted setup (proving key + verifying key) for offline payments.
///
/// Uses OS-level cryptographic randomness. The resulting keys are non-reproducible.
/// Both files must be saved securely — the proving key is distributed to wallets,
/// the verifying key is set on-chain.
pub fn generate_trusted_setup(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let pk_path = matches.value_of("pk-out").unwrap_or("proving_key.bin");
	let vk_path = matches.value_of("vk-out").unwrap_or("verifying_key.bin");

	eprintln!("Generating trusted setup with OS randomness...");
	eprintln!("This may take a few seconds.");

	let setup = {
		use ark_bn254::{Bn254, Fr};
		use ark_groth16::Groth16;
		use ark_snark::SNARK;
		use ark_std::rand::rngs::OsRng;
		use pallet_encointer_offline_payment::circuit::{
			poseidon_config as cfg, OfflinePaymentCircuit,
		};

		let circuit = OfflinePaymentCircuit::new(
			cfg(),
			Fr::from(1u64),
			Fr::from(1u64),
			Fr::from(1u64),
			Fr::from(1u64),
			Fr::from(1u64),
		);
		let (pk, vk) =
			Groth16::<Bn254>::circuit_specific_setup(circuit, &mut OsRng).expect("Setup failed");
		TrustedSetup { proving_key: pk, verifying_key: vk }
	};

	let pk_bytes = setup.proving_key_bytes();
	let vk_bytes = setup.verifying_key_bytes();

	std::fs::write(pk_path, &pk_bytes).expect("Failed to write proving key file");
	std::fs::write(vk_path, &vk_bytes).expect("Failed to write verifying key file");

	let pk_hash = sp_io::hashing::blake2_256(&pk_bytes);
	let vk_hash = sp_io::hashing::blake2_256(&vk_bytes);

	println!("Trusted setup generated successfully.");
	println!();
	println!(
		"  Proving key:    {} ({} bytes, blake2: 0x{})",
		pk_path,
		pk_bytes.len(),
		hex::encode(pk_hash)
	);
	println!(
		"  Verifying key:  {} ({} bytes, blake2: 0x{})",
		vk_path,
		vk_bytes.len(),
		hex::encode(vk_hash)
	);
	println!();
	println!("Next steps:");
	println!(
		"  1. Verify:      encointer-client-notee verify-trusted-setup --pk {} --vk {}",
		pk_path, vk_path
	);
	println!("  2. Set on-chain: encointer-client-notee set-offline-payment-vk --vk-file {} --signer //Alice", vk_path);
	println!("  3. Distribute {} to wallet apps (bundle as asset)", pk_path);

	Ok(())
}

/// Verify that a proving key and verifying key are consistent.
///
/// Generates a test proof with the PK, then verifies it with the VK.
/// If verification succeeds, the keys match and are ready for use.
pub fn verify_trusted_setup(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let pk_path = matches.value_of("pk").expect("--pk is required");
	let vk_path = matches.value_of("vk").expect("--vk is required");

	eprintln!("Loading keys...");

	let pk_bytes = std::fs::read(pk_path).expect("Failed to read proving key file");
	let vk_bytes = std::fs::read(vk_path).expect("Failed to read verifying key file");

	let pk_hash = sp_io::hashing::blake2_256(&pk_bytes);
	let vk_hash = sp_io::hashing::blake2_256(&vk_bytes);

	println!("Proving key:   {} bytes, blake2: 0x{}", pk_bytes.len(), hex::encode(pk_hash));
	println!("Verifying key: {} bytes, blake2: 0x{}", vk_bytes.len(), hex::encode(vk_hash));

	let pk = TrustedSetup::proving_key_from_bytes(&pk_bytes)
		.expect("Failed to deserialize proving key — file may be corrupt");

	let vk = TrustedSetup::verifying_key_from_bytes(&vk_bytes)
		.expect("Failed to deserialize verifying key — file may be corrupt");

	eprintln!("Generating test proof...");

	// Use dummy values for a test proof
	let zk_secret = bytes32_to_field(&[1u8; 32]);
	let nonce = bytes32_to_field(&[2u8; 32]);
	let recipient_hash = bytes32_to_field(&[3u8; 32]);
	let amount = bytes32_to_field(&[4u8; 32]);
	let asset_hash = bytes32_to_field(&[5u8; 32]);

	let (proof, public_inputs) =
		generate_proof(&pk, zk_secret, nonce, recipient_hash, amount, asset_hash)
			.expect("Proof generation failed — proving key may be invalid");

	eprintln!("Verifying proof...");

	if verify_proof(&vk, &proof, &public_inputs) {
		println!();
		println!("PASS: Proving key and verifying key are consistent.");
		println!("      The verifying key is ready to be set on-chain.");
	} else {
		println!();
		println!("FAIL: Proof generated with PK does not verify with VK.");
		println!("      These keys do NOT belong to the same trusted setup.");
		std::process::exit(1);
	}

	Ok(())
}

/// Inspect a proving key or verifying key file.
///
/// Shows metadata: file size, blake2 hash, and validates deserialization.
pub fn inspect_setup_key(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let path = matches.value_of("file").expect("--file is required");

	let bytes = std::fs::read(path).expect("Failed to read file");
	let hash = sp_io::hashing::blake2_256(&bytes);

	println!("File:   {}", path);
	println!("Size:   {} bytes", bytes.len());
	println!("Blake2: 0x{}", hex::encode(hash));

	// Try PK first — a PK embeds a VK, so the VK deserializer would falsely
	// succeed on PK files by reading just the first ~424 bytes.
	if let Some(_pk) = TrustedSetup::proving_key_from_bytes(&bytes) {
		println!("Type:   Proving Key (valid)");
	} else if let Some(_vk) = TrustedSetup::verifying_key_from_bytes(&bytes) {
		println!("Type:   Verifying Key (valid)");
		println!();
		println!("Hex:    {}", hex::encode(&bytes));
	} else {
		println!("Type:   UNKNOWN — could not deserialize as PK or VK");
		std::process::exit(1);
	}

	Ok(())
}
