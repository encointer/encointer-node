use clap::Subcommand;

use super::Cli;

#[derive(Subcommand)]
pub enum AccountCmd {
	/// Import account into key store (creates new or uses supplied seed)
	New {
		/// Seed, mnemonic or SURI
		seed: Option<String>,
	},
	/// List all accounts in keystore
	List,
	/// Print mnemonic phrase for a keystore account
	Export {
		/// AccountId in SS58 format
		account: String,
	},
	/// Send bootstrapping funds to account(s)
	Fund {
		/// Account(s) to fund, SS58 encoded
		#[arg(required = true, num_args = 1..)]
		fundees: Vec<String>,
	},
	/// Poseidon commitment (offline identity) management
	#[command(subcommand)]
	PoseidonCommitment(PoseidonCommitmentCmd),
	/// Bandersnatch public key management
	#[command(subcommand)]
	BandersnatchPubkey(BandersnatchPubkeyCmd),
}

#[derive(Subcommand)]
pub enum PoseidonCommitmentCmd {
	/// Register offline payment identity (ZK commitment)
	Register {
		/// AccountId (SS58)
		account: String,
	},
	/// Get offline identity commitment
	Get {
		/// AccountId (SS58)
		account: String,
	},
}

#[derive(Subcommand)]
pub enum BandersnatchPubkeyCmd {
	/// Register a Bandersnatch public key
	Register {
		/// AccountId (SS58)
		account: String,
		/// Hex-encoded 32-byte Bandersnatch key (auto-derived if omitted)
		#[arg(long)]
		key: Option<String>,
	},
}

impl AccountCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::{frame, keystore};
		match self {
			Self::New { seed } => keystore::new_account(seed.as_deref()),
			Self::List => keystore::list_accounts(),
			Self::Export { account } => keystore::export_secret(account),
			Self::Fund { fundees } => frame::fund(cli, fundees).await,
			Self::PoseidonCommitment(cmd) => cmd.run(cli).await,
			Self::BandersnatchPubkey(cmd) => cmd.run(cli).await,
		}
	}
}

impl PoseidonCommitmentCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::encointer_offline_payment;
		match self {
			Self::Register { account } =>
				encointer_offline_payment::register_offline_identity(cli, account).await,
			Self::Get { account } =>
				encointer_offline_payment::get_offline_identity(cli, account).await,
		}
	}
}

impl BandersnatchPubkeyCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::encointer_reputation_rings;
		match self {
			Self::Register { account, key } =>
				encointer_reputation_rings::register_bandersnatch_key(cli, account, key.as_deref())
					.await,
		}
	}
}
