use crate::{cli_args::EncointerArgsExtractor, utils::get_chain_api};
use clap::ArgMatches;
use encointer_api_client_extension::{CommunitiesApi, TreasuriesApi};

pub fn get_treasury_account(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;

		let maybecid = if let Some(cid) = matches.cid_arg() {
			Some(api.verify_cid(cid, None).await)
		} else {
			None
		};
		let treasury = api.get_community_treasury_account_unchecked(maybecid).await.unwrap();
		// only print plain businesses to be able to parse them in python scripts
		println!("{treasury}");
		Ok(())
	})
	.into()
}
