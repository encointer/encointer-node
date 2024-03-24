use crate::{
	cli_args::EncointerArgsExtractor,
	utils::{
		ensure_payment, get_chain_api,
		keys::{get_accountid_from_str, get_pair_from_str},
	},
};
use clap::ArgMatches;
use encointer_api_client_extension::{
	set_api_extrisic_params_builder, Api, CommunitiesApi, EncointerXt, ParentchainExtrinsicSigner,
};
use encointer_node_notee_runtime::AccountId;
use encointer_primitives::{
	bazaar::{Business, BusinessIdentifier, OfferingData},
	communities::CommunityIdentifier,
};
use parity_scale_codec::Encode;
use sp_core::{sr25519 as sr25519_core, Pair};
use substrate_api_client::{
	ac_compose_macros::{compose_extrinsic, rpc_params},
	rpc::Request,
	SubmitAndWatch, XtStatus,
};

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
		let cid = api
			.verify_cid(matches.cid_arg().expect("please supply argument --cid"), None)
			.await;
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
		let cid = api
			.verify_cid(matches.cid_arg().expect("please supply argument --cid"), None)
			.await;
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
		let cid = api
			.verify_cid(matches.cid_arg().expect("please supply argument --cid"), None)
			.await;
		let offerings = get_offerings_for_business(&api, cid, account).await.unwrap();
		// only print plain offerings to be able to parse them in python scripts
		println!("{offerings:?}");
		Ok(())
	})
	.into()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BazaarCalls {
	CreateBusiness,
	UpdateBusiness,
	CreateOffering,
}

impl ToString for BazaarCalls {
	fn to_string(&self) -> String {
		match self {
			BazaarCalls::CreateBusiness => "create_business".to_string(),
			BazaarCalls::UpdateBusiness => "update_business".to_string(),
			BazaarCalls::CreateOffering => "create_offering".to_string(),
		}
	}
}

async fn send_bazaar_xt(matches: &ArgMatches<'_>, bazaar_call: &BazaarCalls) -> Result<(), ()> {
	let business_owner = matches.account_arg().map(get_pair_from_str).unwrap();

	let mut api = get_chain_api(matches).await;
	api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(
		business_owner.clone(),
	)));
	let cid = api
		.verify_cid(matches.cid_arg().expect("please supply argument --cid"), None)
		.await;
	let ipfs_cid = matches.ipfs_cid_arg().expect("ipfs cid needed");

	let tx_payment_cid_arg = matches.tx_payment_cid_arg();
	set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;
	let xt: EncointerXt<_> =
		compose_extrinsic!(api, "EncointerBazaar", &bazaar_call.to_string(), cid, ipfs_cid)
			.unwrap();
	ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
	// send and watch extrinsic until ready
	let report = api.submit_and_watch_extrinsic_until(xt, XtStatus::Ready).await.unwrap();
	println!(
		"{} for {}. xt-status: '{:?}'",
		bazaar_call.to_string(),
		business_owner.public(),
		report.status
	);
	Ok(())
}
async fn get_businesses(api: &Api, cid: CommunityIdentifier) -> Option<Vec<Business<AccountId>>> {
	api.client()
		.request("encointer_bazaarGetBusinesses", rpc_params![cid])
		.await
		.expect("Could not find any businesses...")
}

async fn get_offerings(api: &Api, cid: CommunityIdentifier) -> Option<Vec<OfferingData>> {
	api.client()
		.request("encointer_bazaarGetOfferings", rpc_params![cid])
		.await
		.expect("Could not find any business offerings...")
}

async fn get_offerings_for_business(
	api: &Api,
	cid: CommunityIdentifier,
	account_id: AccountId,
) -> Option<Vec<OfferingData>> {
	let b_id = BusinessIdentifier::new(cid, account_id);
	api.client()
		.request("encointer_bazaarGetOfferingsForBusiness", rpc_params![b_id])
		.await
		.expect("Could not find any business offerings...")
}
