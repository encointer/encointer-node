use clap::Subcommand;

use super::Cli;

#[derive(Subcommand)]
pub enum IpfsCmd {
	/// Upload file to IPFS via authenticated gateway
	Upload {
		/// Account to authenticate (must be CC holder)
		#[arg(short = 's', long)]
		signer: String,
		/// IPFS auth gateway URL
		#[arg(long, default_value = "http://localhost:5050")]
		gateway: String,
		/// Path to file to upload
		file_path: String,
	},
}

impl IpfsCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::encointer_ipfs;
		match self {
			Self::Upload { signer, gateway, file_path } =>
				encointer_ipfs::ipfs_upload(cli, signer, gateway, file_path).await,
		}
	}
}
