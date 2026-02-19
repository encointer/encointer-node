//! IPFS upload command with sr25519 gateway authentication

use crate::{cli::Cli, exit_code, utils::keys::get_pair_from_str};
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use sp_core::{crypto::Ss58Codec, Pair};
use std::path::Path;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChallengeRequest {
	address: String,
	community_id: String,
}

#[derive(Deserialize)]
struct ChallengeResponse {
	nonce: String,
	timestamp: i64,
	message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VerifyRequest {
	address: String,
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
#[serde(rename_all = "PascalCase")]
struct UploadResponse {
	hash: String,
	#[allow(dead_code)]
	name: String,
	#[allow(dead_code)]
	size: String,
}

pub async fn ipfs_upload(cli: &Cli, signer_str: &str, gateway: &str, file_path: &str) {
	let cid = cli.cid.as_deref().expect("--cid required");

	let pair = get_pair_from_str(signer_str);
	let address = format!("{}", pair.public().to_ss58check());
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
	let signature = format!("0x{}", hex::encode(<_ as AsRef<[u8]>>::as_ref(&sig)));

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
}
