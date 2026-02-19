use clap::Subcommand;

use super::Cli;

#[derive(Subcommand)]
pub enum CeremonyCmd {
	/// Read current ceremony phase
	Phase,
	/// Read current ceremony index
	Index,
	/// Participant-related commands
	#[command(subcommand)]
	Participant(ParticipantCmd),
	/// List assigned meetups
	ListMeetups {
		/// Ceremony index (negative = relative to current)
		#[arg(allow_hyphen_values = true)]
		ceremony_index: Option<i32>,
	},
	/// List attestees
	ListAttestees {
		/// Ceremony index (negative = relative to current)
		#[arg(allow_hyphen_values = true)]
		ceremony_index: Option<i32>,
	},
	/// List reputables
	ListReputables,
	/// Print ceremony statistics as JSON
	Stats {
		/// Ceremony index (negative = relative to current)
		#[arg(long = "ceremony-index", allow_hyphen_values = true)]
		ceremony_index: Option<i32>,
	},
	/// Admin commands (privileged)
	#[command(subcommand)]
	Admin(CeremonyAdminCmd),
}

#[derive(Subcommand)]
pub enum ParticipantCmd {
	/// Register for ceremony
	Register {
		/// AccountId (SS58)
		account: String,
		/// Account which signs the tx
		#[arg(short = 's', long)]
		signer: Option<String>,
	},
	/// Unregister from ceremony
	Unregister {
		/// AccountId (SS58)
		account: String,
		/// Account which signs the tx
		#[arg(short = 's', long)]
		signer: Option<String>,
		/// Ceremony index (negative = relative to current)
		#[arg(allow_hyphen_values = true)]
		ceremony_index: Option<i32>,
	},
	/// Upgrade registration to reputable
	Upgrade {
		/// AccountId (SS58)
		account: String,
		/// Account which signs the tx
		#[arg(short = 's', long)]
		signer: Option<String>,
	},
	/// Endorse newcomers with a bootstrapper account
	Endorse {
		/// Bootstrapper account (SS58)
		bootstrapper: String,
		/// Endorsee account(s) (SS58)
		#[arg(short = 'e', long = "endorsees", required = true, num_args = 1..)]
		endorsees: Vec<String>,
	},
	/// Claim attendance for ceremony
	Attest {
		/// AccountId (SS58)
		account: String,
		/// Attestee accounts (SS58, min 2)
		#[arg(required = true, num_args = 2..)]
		attestees: Vec<String>,
	},
	/// Create attendance claim
	NewClaim {
		/// AccountId (SS58)
		account: String,
		/// Vote on number of people present
		vote: u32,
	},
	/// Claim meetup rewards
	ClaimReward {
		/// Account which signs the tx
		#[arg(short = 's', long)]
		signer: Option<String>,
		/// Meetup index to claim for
		#[arg(long)]
		meetup_index: Option<u64>,
		/// Claim for all meetups
		#[arg(short = 'a', long)]
		all: bool,
	},
	/// List registered participants
	List {
		/// Ceremony index (negative = relative to current)
		#[arg(allow_hyphen_values = true)]
		ceremony_index: Option<i32>,
	},
	/// List reputation history
	Reputation {
		/// AccountId (SS58)
		account: String,
	},
	/// Get proof of attendance
	ProofOfAttendance {
		/// AccountId (SS58)
		account: String,
		/// Ceremony index (negative = relative to current)
		#[arg(long = "ceremony-index", allow_hyphen_values = true)]
		ceremony_index: Option<i32>,
	},
}

#[derive(Subcommand)]
pub enum CeremonyAdminCmd {
	/// Advance to next ceremony phase (ROOT)
	NextPhase {
		/// Account with privileges (sudo or councillor)
		#[arg(short = 's', long)]
		signer: Option<String>,
	},
	/// Get bootstrappers with remaining newbie tickets
	BootstrapperTickets,
	/// Purge ceremony history for a community
	Purge {
		/// First ceremony index to purge
		from_cindex: i32,
		/// Last ceremony index to purge
		to_cindex: i32,
	},
	/// Set meetup time offset
	SetMeetupTimeOffset {
		/// Signed offset in milliseconds
		#[arg(long = "time-offset", allow_hyphen_values = true)]
		time_offset: i32,
	},
}

impl CeremonyCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::{encointer_ceremonies, encointer_scheduler};
		match self {
			Self::Phase => encointer_scheduler::get_phase(cli).await,
			Self::Index => encointer_scheduler::get_cindex(cli).await,
			Self::Participant(cmd) => cmd.run(cli).await,
			Self::ListMeetups { ceremony_index } =>
				encointer_ceremonies::list_meetups(cli, *ceremony_index).await,
			Self::ListAttestees { ceremony_index } =>
				encointer_ceremonies::list_attestees(cli, *ceremony_index).await,
			Self::ListReputables => encointer_ceremonies::list_reputables(cli).await,
			Self::Stats { ceremony_index } =>
				encointer_ceremonies::print_ceremony_stats(cli, *ceremony_index).await,
			Self::Admin(cmd) => cmd.run(cli).await,
		}
	}
}

impl ParticipantCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::encointer_ceremonies;
		match self {
			Self::Register { account, signer } =>
				encointer_ceremonies::register_participant(cli, account, signer.as_deref()).await,
			Self::Unregister { account, signer, ceremony_index } =>
				encointer_ceremonies::unregister_participant(
					cli,
					account,
					signer.as_deref(),
					*ceremony_index,
				)
				.await,
			Self::Upgrade { account, signer } =>
				encointer_ceremonies::upgrade_registration(cli, account, signer.as_deref()).await,
			Self::Endorse { bootstrapper, endorsees } =>
				encointer_ceremonies::endorse(cli, bootstrapper, endorsees).await,
			Self::Attest { account, attestees } =>
				encointer_ceremonies::attest_attendees(cli, account, attestees).await,
			Self::NewClaim { account, vote } =>
				encointer_ceremonies::new_claim(cli, account, *vote).await,
			Self::ClaimReward { signer, meetup_index, all } =>
				encointer_ceremonies::claim_reward(cli, signer.as_deref(), *meetup_index, *all)
					.await,
			Self::List { ceremony_index } =>
				encointer_ceremonies::list_participants(cli, *ceremony_index).await,
			Self::Reputation { account } => encointer_ceremonies::reputation(cli, account).await,
			Self::ProofOfAttendance { account, ceremony_index } =>
				encointer_ceremonies::get_proof_of_attendance(cli, account, *ceremony_index).await,
		}
	}
}

impl CeremonyAdminCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::{encointer_ceremonies, encointer_scheduler};
		match self {
			Self::NextPhase { signer } =>
				encointer_scheduler::next_phase(cli, signer.as_deref()).await,
			Self::BootstrapperTickets =>
				encointer_ceremonies::bootstrappers_with_remaining_newbie_tickets(cli).await,
			Self::Purge { from_cindex, to_cindex } =>
				encointer_ceremonies::purge_community_ceremony(cli, *from_cindex, *to_cindex).await,
			Self::SetMeetupTimeOffset { time_offset } =>
				encointer_ceremonies::set_meetup_time_offset(cli, *time_offset).await,
		}
	}
}
