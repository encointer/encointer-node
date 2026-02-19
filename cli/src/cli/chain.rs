use clap::Subcommand;

use super::Cli;

#[derive(Subcommand)]
pub enum ChainCmd {
	/// Query balance for AccountId (native if no --cid, community currency if --cid)
	Balance {
		/// AccountId in SS58 format
		account: String,
		/// List all community currency balances
		#[arg(short = 'a', long)]
		all: bool,
	},
	/// Transfer funds between accounts
	Transfer {
		/// Sender's AccountId (SS58)
		from: String,
		/// Recipient's AccountId (SS58)
		to: String,
		/// Amount to transfer
		amount: String,
		/// Print encoded call instead of sending
		#[arg(short = 'd', long)]
		dryrun: bool,
	},
	/// Transfer all community currency funds (requires --cid)
	TransferAll {
		/// Sender's AccountId (SS58)
		from: String,
		/// Recipient's AccountId (SS58)
		to: String,
	},
	/// Listen to on-chain events
	Listen {
		/// Exit after N encointer events
		#[arg(short = 'e', long = "await-events")]
		events: Option<u32>,
		/// Exit after N blocks
		#[arg(short = 'b', long = "await-blocks")]
		blocks: Option<u32>,
	},
	/// Query node metadata as JSON
	PrintMetadata,
}

impl ChainCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::{encointer_core, frame};
		match self {
			Self::Balance { account, all } => encointer_core::balance(cli, account, *all).await,
			Self::Transfer { from, to, amount, dryrun } =>
				encointer_core::transfer(cli, from, to, amount, *dryrun).await,
			Self::TransferAll { from, to } => encointer_core::transfer_all(cli, from, to).await,
			Self::Listen { events, blocks } => encointer_core::listen(cli, *events, *blocks).await,
			Self::PrintMetadata => frame::print_metadata(cli).await,
		}
	}
}
