use clap::Subcommand;

use super::Cli;

#[derive(Subcommand)]
pub enum PersonhoodCmd {
	/// Ring computation commands
	#[command(subcommand)]
	Ring(RingCmd),
	/// Produce ring-VRF proof of personhood
	ProveRingMembership {
		/// AccountId (SS58)
		account: String,
		/// Ceremony index
		#[arg(long = "ceremony-index")]
		ceremony_index: u32,
		/// Attendance level (1-5)
		#[arg(long, default_value = "1")]
		level: u8,
		/// Sub-ring index
		#[arg(long = "sub-ring", default_value = "0")]
		sub_ring: u32,
		/// Application context for domain separation (different contexts yield unlinkable pseudonyms)
		#[arg(long, default_value = "encointer-pop")]
		context: String,
	},
	/// Verify ring-VRF proof of personhood
	VerifyRingMembership {
		/// Hex-encoded ring-VRF signature
		#[arg(long)]
		signature: String,
		/// Ceremony index
		#[arg(long = "ceremony-index")]
		ceremony_index: u32,
		/// Attendance level (1-5)
		#[arg(long, default_value = "1")]
		level: u8,
		/// Sub-ring index
		#[arg(long = "sub-ring", default_value = "0")]
		sub_ring: u32,
		/// Application context for domain separation (must match the context used for signing)
		#[arg(long, default_value = "encointer-pop")]
		context: String,
	},
	/// Reputation commitment commands
	#[command(subcommand)]
	Commitment(CommitmentCmd),
}

#[derive(Subcommand)]
pub enum RingCmd {
	/// Initiate ring computation
	Initiate {
		/// AccountId (SS58)
		account: String,
		/// Ceremony index
		#[arg(long = "ceremony-index")]
		ceremony_index: u32,
	},
	/// Continue pending ring computation
	Continue {
		/// AccountId (SS58)
		account: String,
	},
	/// Query ring members
	Get {
		/// Ceremony index
		#[arg(long = "ceremony-index")]
		ceremony_index: u32,
	},
}

#[derive(Subcommand)]
pub enum CommitmentCmd {
	/// List reputation commitments
	List {
		/// Filter by purpose ID
		#[arg(long = "purpose-id")]
		purpose_id: Option<u64>,
	},
	/// List reputation commitment purposes
	Purposes,
}

impl PersonhoodCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::encointer_reputation_rings;
		match self {
			Self::Ring(cmd) => cmd.run(cli).await,
			Self::ProveRingMembership { account, ceremony_index, level, sub_ring, context } =>
				encointer_reputation_rings::prove_personhood(
					cli,
					account,
					*ceremony_index,
					*level,
					*sub_ring,
					context,
				)
				.await,
			Self::VerifyRingMembership {
				signature,
				ceremony_index,
				level,
				sub_ring,
				context,
			} =>
				encointer_reputation_rings::verify_personhood(
					cli,
					signature,
					*ceremony_index,
					*level,
					*sub_ring,
					context,
				)
				.await,
			Self::Commitment(cmd) => cmd.run(cli).await,
		}
	}
}

impl RingCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::encointer_reputation_rings;
		match self {
			Self::Initiate { account, ceremony_index } =>
				encointer_reputation_rings::initiate_rings(cli, account, *ceremony_index).await,
			Self::Continue { account } =>
				encointer_reputation_rings::continue_ring_computation(cli, account).await,
			Self::Get { ceremony_index } =>
				encointer_reputation_rings::get_rings(cli, *ceremony_index).await,
		}
	}
}

impl CommitmentCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::encointer_reputation_commitments;
		match self {
			Self::List { purpose_id } =>
				encointer_reputation_commitments::list_commitments(cli, *purpose_id).await,
			Self::Purposes => encointer_reputation_commitments::list_purposes(cli).await,
		}
	}
}
