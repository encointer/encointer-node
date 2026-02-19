pub mod encointer_bazaar;
pub mod encointer_ceremonies;
pub mod encointer_communities;
pub mod encointer_core;
pub mod encointer_democracy;
pub mod encointer_faucet;
pub mod encointer_ipfs;
pub mod encointer_offline_payment;
pub mod encointer_reputation_commitments;
pub mod encointer_reputation_rings;
pub mod encointer_scheduler;
pub mod encointer_treasuries;
pub mod frame;
pub mod keystore;

use crate::cli::{Cli, Commands};

pub async fn run(cli: &Cli) {
	match &cli.command {
		Commands::Chain(cmd) => cmd.run(cli).await,
		Commands::Account(cmd) => cmd.run(cli).await,
		Commands::Community(cmd) => cmd.run(cli).await,
		Commands::Ceremony(cmd) => cmd.run(cli).await,
		Commands::Democracy(cmd) => cmd.run(cli).await,
		Commands::Bazaar(cmd) => cmd.run(cli).await,
		Commands::Faucet(cmd) => cmd.run(cli).await,
		Commands::Personhood(cmd) => cmd.run(cli).await,
		Commands::OfflinePayment(cmd) => cmd.run(cli).await,
		Commands::Ipfs(cmd) => cmd.run(cli).await,
	}
}
