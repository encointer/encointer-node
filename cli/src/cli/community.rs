use clap::Subcommand;

use super::Cli;

#[derive(Subcommand)]
pub enum CommunityCmd {
	/// Register new community
	New {
		/// Enhanced geojson file specifying the community
		specfile: String,
		/// Account with necessary privileges
		#[arg(short = 's', long)]
		signer: Option<String>,
		/// Print encoded call instead of sending
		#[arg(short = 'd', long)]
		dryrun: bool,
		/// Call wrapping: none|sudo|collective
		#[arg(short = 'w', long = "wrap-call", default_value = "none")]
		wrap_call: String,
		/// Maximum batch size
		#[arg(long = "batch-size", default_value = "100")]
		batch_size: u32,
	},
	/// List all registered communities
	List,
	/// Query total issuance for community (requires --cid)
	Issuance,
	/// Location management commands
	#[command(subcommand)]
	Location(LocationCmd),
	/// Treasury commands
	#[command(subcommand)]
	Treasury(TreasuryCmd),
}

#[derive(Subcommand)]
pub enum LocationCmd {
	/// List all meetup locations for a community
	List,
	/// Register new locations for a community
	Add {
		/// Geojson file with locations as points
		specfile: String,
		/// Account with necessary privileges
		#[arg(short = 's', long)]
		signer: Option<String>,
		/// Print encoded call instead of sending
		#[arg(short = 'd', long)]
		dryrun: bool,
	},
	/// Remove a location for a community
	Remove {
		/// Account with necessary privileges
		#[arg(short = 's', long)]
		signer: Option<String>,
		/// Print encoded call instead of sending
		#[arg(short = 'd', long)]
		dryrun: bool,
		/// Geohash of the location
		#[arg(short = 'g', long)]
		geohash: Option<String>,
		/// Location index to remove
		#[arg(short = 'l', long)]
		location_index: Option<u32>,
	},
}

#[derive(Subcommand)]
pub enum TreasuryCmd {
	/// Get community treasury account
	GetAccount,
	/// Swap option commands
	#[command(subcommand)]
	SwapOption(SwapOptionCmd),
}

#[derive(Subcommand)]
pub enum SwapOptionCmd {
	/// Query swap native option for an account
	GetNative {
		/// AccountId (SS58)
		account: String,
	},
	/// Query swap asset option for an account
	GetAsset {
		/// AccountId (SS58)
		account: String,
	},
	/// Exercise a swap native option
	ExerciseNative {
		/// AccountId (SS58)
		account: String,
		/// Desired amount of native tokens to receive
		amount: u128,
	},
	/// Exercise a swap asset option
	ExerciseAsset {
		/// AccountId (SS58)
		account: String,
		/// Desired amount of asset tokens to receive
		amount: u128,
	},
}

impl CommunityCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::{encointer_communities, encointer_core};
		match self {
			Self::New { specfile, signer, dryrun, wrap_call, batch_size } =>
				encointer_communities::new_community(
					cli,
					specfile,
					signer.as_deref(),
					*dryrun,
					wrap_call,
					*batch_size,
				)
				.await,
			Self::List => encointer_communities::list_communities(cli).await,
			Self::Issuance => encointer_core::issuance(cli).await,
			Self::Location(cmd) => cmd.run(cli).await,
			Self::Treasury(cmd) => cmd.run(cli).await,
		}
	}
}

impl LocationCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::encointer_communities;
		match self {
			Self::List => encointer_communities::list_locations(cli).await,
			Self::Add { specfile, signer, dryrun } =>
				encointer_communities::add_locations(cli, specfile, signer.as_deref(), *dryrun)
					.await,
			Self::Remove { signer, dryrun, geohash, location_index } =>
				encointer_communities::remove_locations(
					cli,
					signer.as_deref(),
					*dryrun,
					geohash.as_deref(),
					*location_index,
				)
				.await,
		}
	}
}

impl TreasuryCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::encointer_treasuries;
		match self {
			Self::GetAccount => encointer_treasuries::get_treasury_account(cli).await,
			Self::SwapOption(cmd) => cmd.run(cli).await,
		}
	}
}

impl SwapOptionCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::encointer_treasuries;
		match self {
			Self::GetNative { account } =>
				encointer_treasuries::get_swap_native_option(cli, account).await,
			Self::GetAsset { account } =>
				encointer_treasuries::get_swap_asset_option(cli, account).await,
			Self::ExerciseNative { account, amount } =>
				encointer_treasuries::swap_native(cli, account, *amount).await,
			Self::ExerciseAsset { account, amount } =>
				encointer_treasuries::swap_asset(cli, account, *amount).await,
		}
	}
}
