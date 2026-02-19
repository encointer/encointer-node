use clap::Subcommand;

use super::Cli;

#[derive(Subcommand)]
pub enum FaucetCmd {
	/// Create a faucet
	Create {
		/// Creator AccountId (SS58)
		account: String,
		/// Faucet name
		faucet_name: String,
		/// Faucet balance
		faucet_balance: u128,
		/// Drip amount
		faucet_drip_amount: u128,
		/// Whitelist of CIDs
		whitelist: Vec<String>,
	},
	/// Drip from a faucet
	Drip {
		/// AccountId (SS58)
		account: String,
		/// Faucet account (SS58)
		faucet_account: String,
		/// Ceremony index
		cindex: i32,
	},
	/// Dissolve a faucet (root only)
	Dissolve {
		/// Account with privileges
		#[arg(short = 's', long)]
		signer: Option<String>,
		/// Faucet account (SS58)
		faucet_account: String,
		/// Beneficiary of remaining funds (SS58)
		faucet_beneficiary: String,
	},
	/// Close an empty faucet
	Close {
		/// Creator AccountId (SS58)
		account: String,
		/// Faucet account (SS58)
		faucet_account: String,
	},
	/// Set faucet reserve amount (root)
	SetReserveAmount {
		/// Account with privileges
		#[arg(short = 's', long)]
		signer: Option<String>,
		/// Reserve amount
		faucet_reserve_amount: u128,
	},
	/// List all faucets
	List,
}

impl FaucetCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::encointer_faucet;
		match self {
			Self::Create {
				account,
				faucet_name,
				faucet_balance,
				faucet_drip_amount,
				whitelist,
			} =>
				encointer_faucet::create_faucet(
					cli,
					account,
					faucet_name,
					*faucet_balance,
					*faucet_drip_amount,
					whitelist,
				)
				.await,
			Self::Drip { account, faucet_account, cindex } =>
				encointer_faucet::drip_faucet(cli, account, faucet_account, *cindex).await,
			Self::Dissolve { signer, faucet_account, faucet_beneficiary } =>
				encointer_faucet::dissolve_faucet(
					cli,
					signer.as_deref(),
					faucet_account,
					faucet_beneficiary,
				)
				.await,
			Self::Close { account, faucet_account } =>
				encointer_faucet::close_faucet(cli, account, faucet_account).await,
			Self::SetReserveAmount { signer, faucet_reserve_amount } =>
				encointer_faucet::set_faucet_reserve_amount(
					cli,
					signer.as_deref(),
					*faucet_reserve_amount,
				)
				.await,
			Self::List => encointer_faucet::list_faucets(cli).await,
		}
	}
}
