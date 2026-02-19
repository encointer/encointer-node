mod account;
mod bazaar;
mod ceremony;
mod chain;
mod community;
mod democracy;
mod faucet;
mod ipfs;
mod offline_payment;
mod personhood;

pub use account::*;
pub use bazaar::*;
pub use ceremony::*;
pub use chain::*;
pub use community::*;
pub use democracy::*;
pub use faucet::*;
pub use ipfs::*;
pub use offline_payment::*;
pub use personhood::*;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
	name = "encointer-cli",
	version,
	author = "Encointer Association <info@encointer.org>",
	about = "interact with encointer-node"
)]
pub struct Cli {
	#[arg(short = 'u', long, global = true, default_value = "ws://127.0.0.1")]
	pub node_url: String,

	#[arg(short = 'p', long, global = true, default_value = "9944")]
	pub node_port: String,

	#[arg(short = 'c', long, global = true, help = "community identifier, base58 encoded")]
	pub cid: Option<String>,

	#[arg(
		long = "tx-payment-cid",
		global = true,
		help = "cid of the community currency in which tx fees should be paid"
	)]
	pub tx_payment_cid: Option<String>,

	#[arg(long = "at", global = true, help = "block hash at which to query")]
	pub at: Option<String>,

	#[arg(short = 'v', long, global = true, help = "print extra information")]
	pub verbose: bool,

	#[command(subcommand)]
	pub command: Commands,
}

impl Cli {
	pub fn at_block(&self) -> Option<sp_core::H256> {
		self.at.as_ref().map(|hex| {
			let vec = sp_core::bytes::from_hex(hex)
				.unwrap_or_else(|_| panic!("bytes::from_hex failed, data is: {hex}"));
			assert!(vec.len() == 32, "block hash must be 32 bytes");
			sp_core::H256::from_slice(&vec)
		})
	}
}

#[derive(Subcommand)]
pub enum Commands {
	#[command(flatten)]
	Chain(ChainCmd),
	/// Account management commands
	#[command(subcommand)]
	Account(AccountCmd),
	/// Community-related commands
	#[command(subcommand)]
	Community(CommunityCmd),
	/// Ceremony-related commands
	#[command(subcommand)]
	Ceremony(CeremonyCmd),
	/// Democracy-related commands
	#[command(subcommand)]
	Democracy(DemocracyCmd),
	/// Bazaar-related commands
	#[command(subcommand)]
	Bazaar(BazaarCmd),
	/// Faucet-related commands
	#[command(subcommand)]
	Faucet(FaucetCmd),
	/// Personhood-related commands
	#[command(subcommand)]
	Personhood(PersonhoodCmd),
	/// Offline payment-related commands
	#[command(subcommand)]
	OfflinePayment(OfflinePaymentCmd),
	/// IPFS-related commands
	#[command(subcommand)]
	Ipfs(IpfsCmd),
}
