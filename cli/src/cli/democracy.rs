use clap::Subcommand;

use super::Cli;

#[derive(Subcommand)]
pub enum DemocracyCmd {
	/// Submit a proposal
	#[command(subcommand)]
	Propose(ProposeCmd),
	/// Proposal queries
	#[command(subcommand)]
	Proposal(ProposalCmd),
	/// List enactment queue
	EnactmentQueue,
	/// Submit a vote
	Vote {
		/// AccountId (SS58)
		account: String,
		/// Proposal ID
		proposal_id: u128,
		/// Vote: aye or nay
		vote: String,
		/// Reputation: cid1_cindex1,cid2_cindex2,...
		reputation_vec: String,
	},
}

#[derive(Subcommand)]
pub enum ProposeCmd {
	/// Submit set inactivity timeout proposal
	SetInactivityTimeout {
		/// AccountId (SS58)
		account: String,
		/// Inactivity timeout value
		inactivity_timeout: u32,
	},
	/// Submit update nominal income proposal
	UpdateNominalIncome {
		/// AccountId (SS58)
		account: String,
		/// New nominal income
		nominal_income: f64,
	},
	/// Submit update demurrage proposal
	UpdateDemurrage {
		/// AccountId (SS58)
		account: String,
		/// Demurrage halving blocks
		demurrage_halving_blocks: u64,
	},
	/// Submit a petition
	Petition {
		/// AccountId (SS58)
		account: String,
		/// What the petition demands
		demand: String,
	},
	/// Submit spend native proposal
	SpendNative {
		/// AccountId (SS58)
		account: String,
		/// Beneficiary (SS58)
		to: String,
		/// Amount
		amount: u128,
	},
	/// Submit proposal to issue a swap native option
	IssueSwapNativeOption {
		/// AccountId (SS58)
		account: String,
		/// Beneficiary (SS58)
		to: String,
		/// Total native token allowance
		#[arg(long = "native-allowance")]
		native_allowance: u128,
		/// CC per native token exchange rate (omit for oracle/auction)
		#[arg(long)]
		rate: Option<f64>,
		/// Burn CC instead of sending to treasury
		#[arg(long = "do-burn")]
		do_burn: bool,
		/// First time of validity (unix timestamp in milliseconds)
		#[arg(long = "valid-from")]
		valid_from: Option<u64>,
		/// Expiry time (unix timestamp in milliseconds)
		#[arg(long = "valid-until")]
		valid_until: Option<u64>,
	},
	/// Submit proposal to issue a swap asset option
	IssueSwapAssetOption {
		/// AccountId (SS58)
		account: String,
		/// Beneficiary (SS58)
		to: String,
		/// SCALE-encoded VersionedLocatableAsset (hex)
		#[arg(long = "asset-id")]
		asset_id: String,
		/// Total asset token allowance
		#[arg(long = "asset-allowance")]
		asset_allowance: u128,
		/// CC per asset token exchange rate (omit for oracle/auction)
		#[arg(long)]
		rate: Option<f64>,
		/// Burn CC instead of sending to treasury
		#[arg(long = "do-burn")]
		do_burn: bool,
		/// First time of validity (unix timestamp in milliseconds)
		#[arg(long = "valid-from")]
		valid_from: Option<u64>,
		/// Expiry time (unix timestamp in milliseconds)
		#[arg(long = "valid-until")]
		valid_until: Option<u64>,
	},
}

#[derive(Subcommand)]
pub enum ProposalCmd {
	/// List proposals
	List {
		/// Include failed proposals
		#[arg(short = 'a', long)]
		all: bool,
	},
	/// Update proposal state
	UpdateState {
		/// AccountId (SS58)
		account: String,
		/// Proposal ID
		proposal_id: u128,
	},
}

impl DemocracyCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::encointer_democracy;
		match self {
			Self::Propose(cmd) => cmd.run(cli).await,
			Self::Proposal(cmd) => cmd.run(cli).await,
			Self::EnactmentQueue => encointer_democracy::list_enactment_queue(cli).await,
			Self::Vote { account, proposal_id, vote, reputation_vec } =>
				encointer_democracy::vote(cli, account, *proposal_id, vote, reputation_vec).await,
		}
	}
}

impl ProposeCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::encointer_democracy;
		match self {
			Self::SetInactivityTimeout { account, inactivity_timeout } =>
				encointer_democracy::submit_set_inactivity_timeout_proposal(
					cli,
					account,
					*inactivity_timeout,
				)
				.await,
			Self::UpdateNominalIncome { account, nominal_income } =>
				encointer_democracy::submit_update_nominal_income_proposal(
					cli,
					account,
					*nominal_income,
				)
				.await,
			Self::UpdateDemurrage { account, demurrage_halving_blocks } =>
				encointer_democracy::submit_update_demurrage_proposal(
					cli,
					account,
					*demurrage_halving_blocks,
				)
				.await,
			Self::Petition { account, demand } =>
				encointer_democracy::submit_petition(cli, account, demand).await,
			Self::SpendNative { account, to, amount } =>
				encointer_democracy::submit_spend_native_proposal(cli, account, to, *amount).await,
			Self::IssueSwapNativeOption {
				account,
				to,
				native_allowance,
				rate,
				do_burn,
				valid_from,
				valid_until,
			} =>
				encointer_democracy::submit_issue_swap_native_option_proposal(
					cli,
					account,
					to,
					*native_allowance,
					*rate,
					*do_burn,
					*valid_from,
					*valid_until,
				)
				.await,
			Self::IssueSwapAssetOption {
				account,
				to,
				asset_id,
				asset_allowance,
				rate,
				do_burn,
				valid_from,
				valid_until,
			} =>
				encointer_democracy::submit_issue_swap_asset_option_proposal(
					cli,
					account,
					to,
					asset_id,
					*asset_allowance,
					*rate,
					*do_burn,
					*valid_from,
					*valid_until,
				)
				.await,
		}
	}
}

impl ProposalCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::encointer_democracy;
		match self {
			Self::List { all } => encointer_democracy::list_proposals(cli, *all).await,
			Self::UpdateState { account, proposal_id } =>
				encointer_democracy::update_proposal_state(cli, account, *proposal_id).await,
		}
	}
}
