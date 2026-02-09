use crate::{
	cli_args::EncointerArgsExtractor,
	utils::{
		get_chain_api,
		keys::{get_accountid_from_str, get_pair_from_str},
	},
};
use clap::ArgMatches;
use encointer_api_client_extension::{
	set_api_extrisic_params_builder, CommunitiesApi, EncointerXt, ParentchainExtrinsicSigner,
};
use encointer_primitives::balances::BalanceType;
use log::info;
use pallet_encointer_offline_payment::{
	compute_commitment, compute_nullifier, derive_zk_secret, OfflinePaymentProof,
};
use parity_scale_codec::Encode;
use sp_core::{crypto::Ss58Codec, sr25519 as sr25519_core, Pair};
use sp_io::hashing::blake2_256;
use substrate_api_client::{
	ac_compose_macros::compose_extrinsic, GetStorage, SubmitAndWatch, XtStatus,
};

/// Register offline identity for an account
pub fn register_offline_identity(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();

		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));

		// Derive zk_secret from the account's seed
		let seed_bytes = who.to_raw_vec();
		let zk_secret = derive_zk_secret(&seed_bytes);
		let commitment = compute_commitment(&zk_secret);

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
					if event.pallet_name() == "EncointerOfflinePayment"
						&& event.variant_name() == "OfflineIdentityRegistered"
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

		Ok(())
	})
	.into()
}

/// Generate an offline payment proof (outputs JSON)
pub fn generate_offline_payment(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;

		let from = matches.signer_arg().map(get_pair_from_str).unwrap();
		let to = get_accountid_from_str(matches.value_of("to").unwrap());
		let amount_str = matches.value_of("amount").unwrap();
		// Validate amount is parseable
		let _: f64 = amount_str.parse().expect("Invalid amount");

		let cid = api
			.verify_cid(matches.cid_arg().expect("please supply argument --cid"), None)
			.await;

		// Derive zk_secret from sender's seed
		let seed_bytes = from.to_raw_vec();
		let zk_secret = derive_zk_secret(&seed_bytes);

		// Generate random nonce
		let nonce: [u8; 32] = blake2_256(&[
			&seed_bytes[..],
			&std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.unwrap()
				.as_nanos()
				.to_le_bytes()[..],
		]
		.concat());

		let nullifier = compute_nullifier(&zk_secret, &nonce);
		let proof = OfflinePaymentProof::new(zk_secret, nonce);

		let sender = get_accountid_from_str(&from.public().to_ss58check());

		// Output as JSON for easy parsing
		let output = serde_json::json!({
			"proof": hex::encode(proof.encode()),
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
		let (proof, sender, recipient, amount, cid, nullifier) =
			if let Some(proof_file) = matches.value_of("proof-file") {
				let content = std::fs::read_to_string(proof_file).expect("Failed to read proof file");
				let json: serde_json::Value =
					serde_json::from_str(&content).expect("Invalid JSON in proof file");

				let proof_bytes =
					hex::decode(json["proof"].as_str().unwrap()).expect("Invalid proof hex");
				let proof: OfflinePaymentProof =
					parity_scale_codec::Decode::decode(&mut &proof_bytes[..])
						.expect("Failed to decode proof");

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

				(proof, sender, recipient, amount, cid, nullifier)
			} else {
				// Parse inline arguments
				let proof_hex = matches.value_of("proof").expect("proof required");
				let proof_bytes = hex::decode(proof_hex).expect("Invalid proof hex");
				let proof: OfflinePaymentProof =
					parity_scale_codec::Decode::decode(&mut &proof_bytes[..])
						.expect("Failed to decode proof");

				let sender = get_accountid_from_str(matches.value_of("sender").unwrap());
				let recipient = get_accountid_from_str(matches.value_of("recipient").unwrap());
				let amount = BalanceType::from_num(
					matches.value_of("amount").unwrap().parse::<f64>().unwrap(),
				);
				let cid = api
					.verify_cid(matches.cid_arg().expect("please supply argument --cid"), None)
					.await;
				let nullifier_bytes =
					hex::decode(matches.value_of("nullifier").unwrap()).expect("Invalid nullifier hex");
				let mut nullifier = [0u8; 32];
				nullifier.copy_from_slice(&nullifier_bytes);

				(proof, sender, recipient, amount, cid, nullifier)
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
