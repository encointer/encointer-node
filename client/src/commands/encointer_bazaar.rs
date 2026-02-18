use crate::{
	cli::Cli,
	utils::{
		ensure_payment, get_chain_api,
		keys::{get_accountid_from_str, get_pair_from_str},
	},
};
use encointer_api_client_extension::{
	set_api_extrisic_params_builder, BazaarApi, CommunitiesApi, EncointerXt,
	ParentchainExtrinsicSigner,
};
use parity_scale_codec::Encode;
use sp_core::{sr25519 as sr25519_core, Pair};
use substrate_api_client::{ac_compose_macros::compose_extrinsic, SubmitAndWatch, XtStatus};

pub async fn create_business(cli: &Cli, account: &str, ipfs_cid: &str) {
	send_bazaar_xt(cli, account, ipfs_cid, &BazaarCalls::CreateBusiness).await;
}

pub async fn update_business(cli: &Cli, account: &str, ipfs_cid: &str) {
	send_bazaar_xt(cli, account, ipfs_cid, &BazaarCalls::UpdateBusiness).await;
}

pub async fn create_offering(cli: &Cli, account: &str, ipfs_cid: &str) {
	send_bazaar_xt(cli, account, ipfs_cid, &BazaarCalls::CreateOffering).await;
}

pub async fn list_businesses(cli: &Cli) {
	let api = get_chain_api(cli).await;
	let cid = api
		.verify_cid(cli.cid.as_deref().expect("please supply argument --cid"), None)
		.await;
	let businesses = api.get_businesses(cid).await.unwrap();
	// only print plain businesses to be able to parse them in python scripts
	println!("{businesses:?}");
}

pub async fn list_offerings(cli: &Cli) {
	let api = get_chain_api(cli).await;
	let cid = api
		.verify_cid(cli.cid.as_deref().expect("please supply argument --cid"), None)
		.await;
	let offerings = api.get_offerings(cid).await.unwrap();
	// only print plain offerings to be able to parse them in python scripts
	println!("{offerings:?}");
}

pub async fn list_business_offerings(cli: &Cli, account: &str) {
	let account = get_accountid_from_str(account);
	let api = get_chain_api(cli).await;
	let cid = api
		.verify_cid(cli.cid.as_deref().expect("please supply argument --cid"), None)
		.await;
	let offerings = api.get_offerings_for_business(cid, account).await.unwrap();
	// only print plain offerings to be able to parse them in python scripts
	println!("{offerings:?}");
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

async fn send_bazaar_xt(cli: &Cli, account: &str, ipfs_cid: &str, bazaar_call: &BazaarCalls) {
	let business_owner = get_pair_from_str(account);

	let mut api = get_chain_api(cli).await;
	api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(
		business_owner.clone(),
	)));
	let cid = api
		.verify_cid(cli.cid.as_deref().expect("please supply argument --cid"), None)
		.await;

	let tx_payment_cid_arg = cli.tx_payment_cid.as_deref();
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
}
