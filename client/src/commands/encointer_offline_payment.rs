use crate::{
	cli::Cli,
	utils::{
		contains_sudo_pallet, get_chain_api,
		keys::{get_accountid_from_str, get_pair_from_str},
		print_raw_call, send_and_wait_for_in_block, sudo_call, xt, OpaqueCall,
	},
};
use encointer_api_client_extension::{
	set_api_extrisic_params_builder, CommunitiesApi, EncointerXt, ParentchainExtrinsicSigner,
};
use encointer_primitives::balances::BalanceType;
use frame_support::BoundedVec;
use log::info;
use pallet_encointer_offline_payment::{
	ceremony::{
		ceremony_contribute, ceremony_init, serialize_delta_g2, serialize_pk, serialize_vk,
		verify_ceremony_pk, verify_contribution, ContributionReceipt,
	},
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
pub async fn register_offline_identity(cli: &Cli, account: &str) {
	let who = get_pair_from_str(account);

	let mut api = get_chain_api(cli).await;
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

	let tx_payment_cid_arg = cli.tx_payment_cid.as_deref();
	set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

	let xt: EncointerXt<_> =
		compose_extrinsic!(api, "EncointerOfflinePayment", "register_offline_identity", commitment)
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
}

/// Get offline identity for an account
pub async fn get_offline_identity(cli: &Cli, account_str: &str) {
	let api = get_chain_api(cli).await;
	let account = get_accountid_from_str(account_str);

	let maybe_at = cli.at_block();

	let commitment: Option<[u8; 32]> = api
		.get_storage_map("EncointerOfflinePayment", "OfflineIdentities", account.clone(), maybe_at)
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
}

/// Generate an offline payment with real Groth16 ZK proof
pub async fn generate_offline_payment(
	cli: &Cli,
	signer_arg: Option<&str>,
	to_str: &str,
	amount_str: &str,
	pk_file: Option<&str>,
) {
	let api = get_chain_api(cli).await;

	let from = get_pair_from_str(signer_arg.expect("--signer required"));
	let to = get_accountid_from_str(to_str);
	let amount_f64: f64 = amount_str.parse().expect("Invalid amount");
	let amount = BalanceType::from_num(amount_f64);

	let cid = api
		.verify_cid(cli.cid.as_deref().expect("please supply argument --cid"), None)
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
	let pk_ref = if let Some(pk_path) = pk_file {
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
	let (proof, public_inputs) =
		generate_proof(pk_ref, zk_secret, nonce, recipient_hash, amount_field, chain_asset_hash)
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
}

/// Submit an offline payment proof for settlement
pub async fn submit_offline_payment(
	cli: &Cli,
	signer_arg: Option<&str>,
	proof_file: Option<&str>,
	proof_hex: Option<&str>,
	sender_str: Option<&str>,
	recipient_str: Option<&str>,
	amount_str: Option<&str>,
	nullifier_hex: Option<&str>,
) {
	let signer = get_pair_from_str(signer_arg.expect("--signer required"));

	let mut api = get_chain_api(cli).await;
	api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(signer)));

	// Parse proof file or inline arguments
	let (proof_bytes, sender, recipient, amount, cid, nullifier) = if let Some(proof_file) =
		proof_file
	{
		let content = std::fs::read_to_string(proof_file).expect("Failed to read proof file");
		let json: serde_json::Value =
			serde_json::from_str(&content).expect("Invalid JSON in proof file");

		let proof_bytes = hex::decode(json["proof"].as_str().unwrap()).expect("Invalid proof hex");

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
		let proof_bytes =
			hex::decode(proof_hex.expect("proof required")).expect("Invalid proof hex");

		let sender = get_accountid_from_str(sender_str.expect("sender required"));
		let recipient = get_accountid_from_str(recipient_str.expect("recipient required"));
		let amount =
			BalanceType::from_num(amount_str.expect("amount required").parse::<f64>().unwrap());
		let cid = api
			.verify_cid(cli.cid.as_deref().expect("please supply argument --cid"), None)
			.await;
		let nullifier_bytes =
			hex::decode(nullifier_hex.expect("nullifier required")).expect("Invalid nullifier hex");
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

	let tx_payment_cid_arg = cli.tx_payment_cid.as_deref();
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
}

/// Set the Groth16 verification key via sudo (requires --signer to be sudo key)
pub async fn set_verification_key(
	cli: &Cli,
	signer_arg: Option<&str>,
	vk_file: Option<&str>,
	vk_hex: Option<&str>,
) {
	// Use Alice as default signer (sudo in dev mode)
	let signer = signer_arg
		.map_or_else(|| AccountKeyring::Alice.pair(), |signer| get_pair_from_str(signer).into());

	let mut api = get_chain_api(cli).await;
	let signer = ParentchainExtrinsicSigner::new(signer);
	api.set_signer(signer);

	// Load VK from file, hex string, or generate test key
	let vk_bytes = if let Some(file_path) = vk_file {
		let bytes = std::fs::read(file_path).expect("Failed to read VK file");
		TrustedSetup::verifying_key_from_bytes(&bytes)
			.expect("Failed to deserialize VK from file — is it a valid verifying key?");
		eprintln!("Loaded verification key from {} ({} bytes)", file_path, bytes.len());
		bytes
	} else if let Some(hex_str) = vk_hex {
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
		return;
	};

	let tx_payment_cid_arg = cli.tx_payment_cid.as_deref();
	set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

	send_and_wait_for_in_block(&api, xt(&api, call).await, tx_payment_cid_arg).await;

	println!("Verification key set successfully!");
	println!("VK size: {} bytes", vk_bytes.len());
}

/// Generate and output the test verification key
pub fn generate_test_vk() {
	eprintln!("Generating test verification key with seed {}...", TEST_SETUP_SEED);
	let setup = TrustedSetup::generate_with_seed(TEST_SETUP_SEED);
	let vk_bytes = setup.verifying_key_bytes();

	println!("{}", hex::encode(&vk_bytes));
}

/// Generate a trusted setup (proving key + verifying key) for offline payments.
///
/// Uses OS-level cryptographic randomness. The resulting keys are non-reproducible.
/// Both files must be saved securely — the proving key is distributed to wallets,
/// the verifying key is set on-chain.
pub fn generate_trusted_setup(pk_path: &str, vk_path: &str) {
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
		"  1. Verify:      encointer-client-notee offline-payment verify-trusted-setup --pk {} --vk {}",
		pk_path, vk_path
	);
	println!("  2. Set on-chain: encointer-client-notee offline-payment set-vk --vk-file {} --signer //Alice", vk_path);
	println!("  3. Distribute {} to wallet apps (bundle as asset)", pk_path);
}

/// Verify that a proving key and verifying key are consistent.
///
/// Generates a test proof with the PK, then verifies it with the VK.
/// If verification succeeds, the keys match and are ready for use.
pub fn verify_trusted_setup(pk_path: &str, vk_path: &str) {
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
}

/// Inspect a proving key or verifying key file.
///
/// Shows metadata: file size, blake2 hash, and validates deserialization.
pub fn inspect_setup_key(path: &str) {
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
}

// ---------------------------------------------------------------------------
//  Multiparty trusted setup ceremony commands
// ---------------------------------------------------------------------------

/// Initialize a ceremony — generates the initial CRS and an empty transcript.
pub fn cmd_ceremony_init(pk_path: &str, transcript_path: &str) {
	eprintln!("Generating initial CRS with OS randomness...");
	let pk = ceremony_init();

	let pk_bytes = serialize_pk(&pk);
	std::fs::write(pk_path, &pk_bytes).expect("write PK file");

	let pk_hash = sp_io::hashing::blake2_256(&pk_bytes);
	let delta_bytes = serialize_delta_g2(&pk);
	let delta_hash = hex::encode(sp_io::hashing::blake2_256(&delta_bytes));

	let transcript = serde_json::json!({
		"contributions": [],
		"initial_delta_g2_hash": format!("0x{}", delta_hash),
	});
	std::fs::write(transcript_path, serde_json::to_string_pretty(&transcript).unwrap())
		.expect("write transcript");

	println!("Ceremony initialized.");
	println!("  PK: {} ({} bytes, blake2: 0x{})", pk_path, pk_bytes.len(), hex::encode(pk_hash));
	println!("  Transcript: {}", transcript_path);
	println!();
	println!("Next: distribute {} and {} to the first participant.", pk_path, transcript_path);
}

/// Apply one contribution to the ceremony proving key.
pub fn cmd_ceremony_contribute(pk_path: &str, transcript_path: &str, participant: &str) {
	eprintln!("Loading ceremony PK from {}...", pk_path);
	let pk_bytes = std::fs::read(pk_path).expect("read PK");
	let pk = TrustedSetup::proving_key_from_bytes(&pk_bytes)
		.expect("Failed to deserialize PK — file may be corrupt");

	eprintln!("Applying contribution for '{}'...", participant);
	let (pk_new, receipt) = ceremony_contribute(pk);

	// Self-verify
	if !verify_contribution(&receipt) {
		eprintln!("ERROR: Self-verification of receipt FAILED (should not happen)");
		std::process::exit(1);
	}
	eprintln!("Receipt pairing check: PASS");

	eprintln!("Functional test (generate + verify proof)...");
	if !verify_ceremony_pk(&pk_new) {
		eprintln!("ERROR: Functional test FAILED after contribution");
		std::process::exit(1);
	}
	eprintln!("Functional test: PASS");

	// Serialize updated PK
	let new_pk_bytes = serialize_pk(&pk_new);
	std::fs::write(pk_path, &new_pk_bytes).expect("write PK");

	// Update transcript
	let transcript_str = std::fs::read_to_string(transcript_path).expect("read transcript");
	let mut transcript: serde_json::Value =
		serde_json::from_str(&transcript_str).expect("parse transcript");

	let receipt_hex = hex::encode(receipt.to_bytes());
	let receipt_bytes = receipt.to_bytes();
	// Receipt layout: d_g1 (32B) | old_delta_g2 (64B) | new_delta_g2 (64B)
	// old_delta_g2 starts at offset 32, new_delta_g2 starts at offset 96
	let old_delta_hash = hex::encode(sp_io::hashing::blake2_256(&receipt_bytes[32..96]));
	let new_delta_hash = hex::encode(sp_io::hashing::blake2_256(&receipt_bytes[96..160]));

	transcript["contributions"].as_array_mut().expect("contributions array").push(
		serde_json::json!({
			"participant": participant,
			"receipt": receipt_hex,
			"old_delta_g2_hash": format!("0x{}", old_delta_hash),
			"new_delta_g2_hash": format!("0x{}", new_delta_hash),
		}),
	);

	std::fs::write(transcript_path, serde_json::to_string_pretty(&transcript).unwrap())
		.expect("write transcript");

	let pk_hash = sp_io::hashing::blake2_256(&new_pk_bytes);
	println!("Contribution by '{}' applied successfully.", participant);
	println!(
		"  PK: {} ({} bytes, blake2: 0x{})",
		pk_path,
		new_pk_bytes.len(),
		hex::encode(pk_hash)
	);
}

/// Verify all contributions in a ceremony transcript.
pub fn cmd_ceremony_verify(pk_path: &str, transcript_path: &str) {
	let transcript_str = std::fs::read_to_string(transcript_path).expect("read transcript");
	let transcript: serde_json::Value =
		serde_json::from_str(&transcript_str).expect("parse transcript");

	let contributions = transcript["contributions"].as_array().expect("contributions array");
	if contributions.is_empty() {
		println!("No contributions in transcript.");
		return;
	}

	println!("Verifying {} contribution(s)...", contributions.len());

	let mut all_pass = true;
	// Track the previous receipt's new_delta_g2 bytes for chain checks
	let mut prev_new_delta_bytes: Option<Vec<u8>> = None;

	for (i, entry) in contributions.iter().enumerate() {
		let participant = entry["participant"].as_str().unwrap_or("unknown");
		let receipt_hex = entry["receipt"].as_str().expect("receipt field");
		let receipt_bytes = hex::decode(receipt_hex).expect("decode receipt hex");
		let receipt = ContributionReceipt::from_bytes(&receipt_bytes).expect("deserialize receipt");

		// Chain check: previous new_delta_g2 == this old_delta_g2
		// Receipt layout: d_g1 (32B) | old_delta_g2 (64B) | new_delta_g2 (64B)
		if let Some(ref prev) = prev_new_delta_bytes {
			if *prev != receipt_bytes[32..96] {
				println!(
					"  #{} {}: FAIL (chain break — old_delta_g2 mismatch)",
					i + 1,
					participant
				);
				all_pass = false;
				continue;
			}
		}

		let pairing_ok = verify_contribution(&receipt);
		let status = if pairing_ok { "PASS" } else { "FAIL" };
		println!("  #{} {}: {}", i + 1, participant, status);
		if !pairing_ok {
			all_pass = false;
		}

		prev_new_delta_bytes = Some(receipt_bytes[96..160].to_vec());
	}

	// Functional test on final PK
	eprintln!("Loading final PK for functional test...");
	let pk_bytes = std::fs::read(pk_path).expect("read PK");
	let pk = TrustedSetup::proving_key_from_bytes(&pk_bytes).expect("deserialize PK");

	let functional_ok = verify_ceremony_pk(&pk);
	println!("  Functional test: {}", if functional_ok { "PASS" } else { "FAIL" });
	if !functional_ok {
		all_pass = false;
	}

	println!();
	if all_pass {
		println!("PASS: All {} contribution(s) verified.", contributions.len());
	} else {
		println!("FAIL: One or more verifications failed.");
		std::process::exit(1);
	}
}

/// Finalize a ceremony — extract PK and VK files from the ceremony state.
pub fn cmd_ceremony_finalize(pk_in: &str, pk_out: &str, vk_out: &str) {
	eprintln!("Loading ceremony PK from {}...", pk_in);
	let pk_bytes = std::fs::read(pk_in).expect("read PK");
	let pk = TrustedSetup::proving_key_from_bytes(&pk_bytes).expect("deserialize PK");

	let final_pk_bytes = serialize_pk(&pk);
	std::fs::write(pk_out, &final_pk_bytes).expect("write PK");

	let vk_bytes = serialize_vk(&pk);
	std::fs::write(vk_out, &vk_bytes).expect("write VK");

	let pk_hash = sp_io::hashing::blake2_256(&final_pk_bytes);
	let vk_hash = sp_io::hashing::blake2_256(&vk_bytes);

	println!("Ceremony finalized.");
	println!(
		"  Proving key:   {} ({} bytes, blake2: 0x{})",
		pk_out,
		final_pk_bytes.len(),
		hex::encode(pk_hash)
	);
	println!(
		"  Verifying key: {} ({} bytes, blake2: 0x{})",
		vk_out,
		vk_bytes.len(),
		hex::encode(vk_hash)
	);
	println!();
	println!("Next steps:");
	println!(
		"  1. Verify:       encointer-client-notee offline-payment verify-trusted-setup --pk {} --vk {}",
		pk_out, vk_out
	);
	println!(
		"  2. Set on-chain: encointer-client-notee offline-payment set-vk --vk-file {} --signer //Alice",
		vk_out
	);
	println!("  3. Distribute {} to wallet apps", pk_out);
}
