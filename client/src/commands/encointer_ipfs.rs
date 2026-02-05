//! IPFS upload command with sr25519 gateway authentication

use crate::{cli_args::EncointerArgsExtractor, exit_code, utils::keys::get_accountid_from_str};
use clap::ArgMatches;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use sp_core::{crypto::Ss58Codec, sr25519 as sr25519_core, Pair};
use std::path::Path;

fn get_pair_from_suri(account: &str) -> sr25519_core::Pair {
	match &account[..2] {
		"//" => sr25519_core::Pair::from_string(account, None).unwrap(),
		"0x" => sr25519_core::Pair::from_string_with_seed(account, None).unwrap().0,
		_ => {
			if sr25519_core::Public::from_ss58check(account).is_err() {
				return sr25519_core::Pair::from_string_with_seed(account, None).unwrap().0;
			}
			panic!("keystore lookup not supported for ipfs-upload, use suri or dev account")
		},
	}
}

#[derive(Serialize)]
struct ChallengeRequest {
	address: String,
	#[serde(rename = "communityId")]
	community_id: String,
}

#[derive(Deserialize)]
struct ChallengeResponse {
	nonce: String,
	timestamp: i64,
	message: String,
}

#[derive(Serialize)]
struct VerifyRequest {
	address: String,
	#[serde(rename = "communityId")]
	community_id: String,
	signature: String,
	nonce: String,
	timestamp: i64,
}

#[derive(Deserialize)]
struct VerifyResponse {
	token: String,
	#[allow(dead_code)]
	expires_at: i64,
}

#[derive(Deserialize)]
struct UploadResponse {
	#[serde(rename = "Hash")]
	hash: String,
	#[allow(dead_code)]
	#[serde(rename = "Name")]
	name: String,
	#[allow(dead_code)]
	#[serde(rename = "Size")]
	size: String,
}

pub fn ipfs_upload(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async { do_ipfs_upload(matches).await })
}

async fn do_ipfs_upload(matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let gateway = matches.gateway_url_arg().unwrap_or("http://localhost:5050");
	let signer_str = matches.signer_arg().expect("--signer required");
	let cid = matches.cid_arg().expect("--cid required");
	let file_path = matches.file_path_arg().expect("file path required");

	let pair = get_pair_from_suri(signer_str);
	let address = format!("{}", get_accountid_from_str(signer_str));
	let client = reqwest::Client::new();

	// Request challenge
	let challenge_resp = client
		.post(format!("{}/auth/challenge", gateway))
		.json(&ChallengeRequest { address: address.clone(), community_id: cid.to_string() })
		.send()
		.await
		.map_err(|e| {
			eprintln!("Failed to request challenge: {}", e);
			std::process::exit(exit_code::RPC_ERROR);
		})
		.unwrap();

	if !challenge_resp.status().is_success() {
		eprintln!("Challenge request failed: {}", challenge_resp.status());
		std::process::exit(exit_code::RPC_ERROR);
	}

	let challenge: ChallengeResponse = challenge_resp
		.json()
		.await
		.map_err(|e| {
			eprintln!("Failed to parse challenge response: {}", e);
			std::process::exit(exit_code::RPC_ERROR);
		})
		.unwrap();

	// Sign message
	let sig = pair.sign(challenge.message.as_bytes());
	let signature = format!("0x{}", hex::encode(sig.0));

	// Verify and get JWT
	let verify_resp = client
		.post(format!("{}/auth/verify", gateway))
		.json(&VerifyRequest {
			address: address.clone(),
			community_id: cid.to_string(),
			signature,
			nonce: challenge.nonce,
			timestamp: challenge.timestamp,
		})
		.send()
		.await
		.map_err(|e| {
			eprintln!("Failed to verify: {}", e);
			std::process::exit(exit_code::RPC_ERROR);
		})
		.unwrap();

	if verify_resp.status() == 403 {
		eprintln!("Not a CC holder for community {}", cid);
		std::process::exit(exit_code::NOT_CC_HOLDER);
	}

	if !verify_resp.status().is_success() {
		eprintln!("Verify request failed: {}", verify_resp.status());
		std::process::exit(exit_code::RPC_ERROR);
	}

	let token: VerifyResponse = verify_resp
		.json()
		.await
		.map_err(|e| {
			eprintln!("Failed to parse verify response: {}", e);
			std::process::exit(exit_code::RPC_ERROR);
		})
		.unwrap();

	// Upload file
	let file_bytes = std::fs::read(file_path)
		.map_err(|e| {
			eprintln!("Failed to read file: {}", e);
			std::process::exit(1);
		})
		.unwrap();

	let filename = Path::new(file_path).file_name().unwrap().to_str().unwrap();
	let form = multipart::Form::new()
		.part("file", multipart::Part::bytes(file_bytes).file_name(filename.to_string()));

	let upload_resp = client
		.post(format!("{}/ipfs/add", gateway))
		.bearer_auth(&token.token)
		.multipart(form)
		.send()
		.await
		.map_err(|e| {
			eprintln!("Failed to upload: {}", e);
			std::process::exit(exit_code::RPC_ERROR);
		})
		.unwrap();

	if !upload_resp.status().is_success() {
		eprintln!("Upload failed: {}", upload_resp.status());
		std::process::exit(exit_code::RPC_ERROR);
	}

	let result: UploadResponse = upload_resp
		.json()
		.await
		.map_err(|e| {
			eprintln!("Failed to parse upload response: {}", e);
			std::process::exit(exit_code::RPC_ERROR);
		})
		.unwrap();

	println!("{}", result.hash);
	Ok(())
}
