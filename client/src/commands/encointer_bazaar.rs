use crate::cli_args::EncointerArgsExtractor;
use crate::utils::keys::get_accountid_from_str;
use crate::{
	get_businesses, get_chain_api, get_offerings, get_offerings_for_business, send_bazaar_xt,
	verify_cid, BazaarCalls,
};
use clap::ArgMatches;
use encointer_api_client_extension::ExtrinsicAddress;

pub fn create_business(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		send_bazaar_xt(matches, &BazaarCalls::CreateBusiness).await.unwrap();
		Ok(())
	})
	.into()
}
pub fn update_business(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		send_bazaar_xt(matches, &BazaarCalls::UpdateBusiness).await.unwrap();
		Ok(())
	})
	.into()
}
pub fn create_offering(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		send_bazaar_xt(matches, &BazaarCalls::CreateOffering).await.unwrap();
		Ok(())
	})
	.into()
}
pub fn list_businesses(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let cid =
			verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;
		let businesses = get_businesses(&api, cid).await.unwrap();
		// only print plain businesses to be able to parse them in python scripts
		println!("{businesses:?}");
		Ok(())
	})
	.into()
}
pub fn list_offerings(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let cid =
			verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;
		let offerings = get_offerings(&api, cid).await.unwrap();
		// only print plain offerings to be able to parse them in python scripts
		println!("{offerings:?}");
		Ok(())
	})
	.into()
}

pub fn list_business_offerings(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let account = matches.account_arg().map(get_accountid_from_str).unwrap();
		let api = get_chain_api(matches).await;
		let cid =
			verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;
		let offerings = get_offerings_for_business(&api, cid, account).await.unwrap();
		// only print plain offerings to be able to parse them in python scripts
		println!("{offerings:?}");
		Ok(())
	})
	.into()
}
