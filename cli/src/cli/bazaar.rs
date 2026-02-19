use clap::Subcommand;

use super::Cli;

#[derive(Subcommand)]
pub enum BazaarCmd {
	/// Business management commands
	#[command(subcommand)]
	Business(BusinessCmd),
	/// Offering management commands
	#[command(subcommand)]
	Offering(OfferingCmd),
}

#[derive(Subcommand)]
pub enum BusinessCmd {
	/// Register a community business
	Create {
		/// Business owner AccountId (SS58)
		account: String,
		/// IPFS content identifier
		#[arg(long = "ipfs-cid")]
		ipfs_cid: String,
	},
	/// Update an existing community business
	Update {
		/// Business owner AccountId (SS58)
		account: String,
		/// IPFS content identifier
		#[arg(long = "ipfs-cid")]
		ipfs_cid: String,
	},
	/// List businesses for a community
	List,
	/// List offerings for a specific business
	Offerings {
		/// Business owner AccountId (SS58)
		account: String,
	},
}

#[derive(Subcommand)]
pub enum OfferingCmd {
	/// Create an offering for a business
	Create {
		/// Business owner AccountId (SS58)
		account: String,
		/// IPFS content identifier
		#[arg(long = "ipfs-cid")]
		ipfs_cid: String,
	},
	/// List offerings for a community
	List,
}

impl BazaarCmd {
	pub async fn run(&self, cli: &Cli) {
		match self {
			Self::Business(cmd) => cmd.run(cli).await,
			Self::Offering(cmd) => cmd.run(cli).await,
		}
	}
}

impl BusinessCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::encointer_bazaar;
		match self {
			Self::Create { account, ipfs_cid } =>
				encointer_bazaar::create_business(cli, account, ipfs_cid).await,
			Self::Update { account, ipfs_cid } =>
				encointer_bazaar::update_business(cli, account, ipfs_cid).await,
			Self::List => encointer_bazaar::list_businesses(cli).await,
			Self::Offerings { account } =>
				encointer_bazaar::list_business_offerings(cli, account).await,
		}
	}
}

impl OfferingCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::encointer_bazaar;
		match self {
			Self::Create { account, ipfs_cid } =>
				encointer_bazaar::create_offering(cli, account, ipfs_cid).await,
			Self::List => encointer_bazaar::list_offerings(cli).await,
		}
	}
}
