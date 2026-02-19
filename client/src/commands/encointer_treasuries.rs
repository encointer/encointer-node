use crate::{
	cli::Cli,
	utils::{
		ensure_payment, get_chain_api,
		keys::{get_accountid_from_str, get_pair_from_str},
	},
};
use encointer_api_client_extension::{
	set_api_extrisic_params_builder, CommunitiesApi, EncointerXt, Moment,
	ParentchainExtrinsicSigner, TreasuriesApi,
};
use encointer_node_runtime::{AccountId, Balance};
use encointer_primitives::{
	communities::CommunityIdentifier,
	treasuries::{SwapAssetOption, SwapNativeOption},
};
use parity_scale_codec::Encode;
use sp_core::sr25519 as sr25519_core;
use substrate_api_client::{
	ac_compose_macros::compose_extrinsic, GetStorage, SubmitAndWatch, XtStatus,
};

pub async fn get_treasury_account(cli: &Cli) {
	let api = get_chain_api(cli).await;

	let maybecid = if let Some(cid) = cli.cid.as_deref() {
		Some(api.verify_cid(cid, None).await)
	} else {
		None
	};
	let treasury = api.get_community_treasury_account_unchecked(maybecid).await.unwrap();
	println!("{treasury}");
}

pub async fn get_swap_native_option(cli: &Cli, account: &str) {
	let api = get_chain_api(cli).await;
	let cid = api
		.verify_cid(cli.cid.as_deref().expect("please supply argument --cid"), None)
		.await;
	let account = get_accountid_from_str(account);
	let maybe_at = cli.at_block();
	let option: Option<SwapNativeOption<Balance, Moment>> = api
		.get_storage_double_map(
			"EncointerTreasuries",
			"SwapNativeOptions",
			cid,
			&account,
			maybe_at,
		)
		.await
		.unwrap();
	match option {
		Some(opt) => print_swap_native_option(&opt),
		None => println!("No swap native option found for {account} in {cid}"),
	}
}

pub async fn get_swap_asset_option(cli: &Cli, account: &str) {
	let api = get_chain_api(cli).await;
	let cid = api
		.verify_cid(cli.cid.as_deref().expect("please supply argument --cid"), None)
		.await;
	let account = get_accountid_from_str(account);
	let maybe_at = cli.at_block();
	use super::encointer_democracy::XcmLocation;
	let option: Option<SwapAssetOption<Balance, Moment, XcmLocation>> = api
		.get_storage_double_map(
			"EncointerTreasuries",
			"SwapAssetOptions",
			cid,
			&account,
			maybe_at,
		)
		.await
		.unwrap();
	match option {
		Some(opt) => print_swap_asset_option(&opt),
		None => println!("No swap asset option found for {account} in {cid}"),
	}
}

pub async fn swap_native(cli: &Cli, account: &str, amount: u128) {
	let who = get_pair_from_str(account);
	let mut api = get_chain_api(cli).await;
	api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));
	let cid = api
		.verify_cid(cli.cid.as_deref().expect("please supply argument --cid"), None)
		.await;
	let tx_payment_cid_arg = cli.tx_payment_cid.as_deref();
	set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

	let xt: EncointerXt<_> =
		compose_extrinsic!(api, "EncointerTreasuries", "swap_native", cid, amount).unwrap();
	ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
	let _result = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await;
	println!("Swap native submitted: {amount} from community {cid}");
}

pub async fn swap_asset(cli: &Cli, account: &str, amount: u128) {
	let who = get_pair_from_str(account);
	let mut api = get_chain_api(cli).await;
	api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));
	let cid = api
		.verify_cid(cli.cid.as_deref().expect("please supply argument --cid"), None)
		.await;
	let tx_payment_cid_arg = cli.tx_payment_cid.as_deref();
	set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

	let xt: EncointerXt<_> =
		compose_extrinsic!(api, "EncointerTreasuries", "swap_asset", cid, amount).unwrap();
	ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
	let _result = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await;
	println!("Swap asset submitted: {amount} from community {cid}");
}

fn print_swap_native_option(opt: &SwapNativeOption<Balance, Moment>) {
	println!("SwapNativeOption:");
	println!("  cid: {}", opt.cid);
	println!("  native_allowance: {}", opt.native_allowance);
	println!(
		"  rate: {}",
		opt.rate
			.map(|r| format!("{r}"))
			.unwrap_or_else(|| "None (oracle/auction)".into())
	);
	println!("  do_burn: {}", opt.do_burn);
	println!(
		"  valid_from: {}",
		opt.valid_from.map(|t| format!("{t}")).unwrap_or_else(|| "None".into())
	);
	println!(
		"  valid_until: {}",
		opt.valid_until.map(|t| format!("{t}")).unwrap_or_else(|| "None".into())
	);
}

fn print_swap_asset_option<AssetId: core::fmt::Debug>(
	opt: &SwapAssetOption<Balance, Moment, AssetId>,
) {
	println!("SwapAssetOption:");
	println!("  cid: {}", opt.cid);
	println!("  asset_id: {:?}", opt.asset_id);
	println!("  asset_allowance: {}", opt.asset_allowance);
	println!(
		"  rate: {}",
		opt.rate
			.map(|r| format!("{r}"))
			.unwrap_or_else(|| "None (oracle/auction)".into())
	);
	println!("  do_burn: {}", opt.do_burn);
	println!(
		"  valid_from: {}",
		opt.valid_from.map(|t| format!("{t}")).unwrap_or_else(|| "None".into())
	);
	println!(
		"  valid_until: {}",
		opt.valid_until.map(|t| format!("{t}")).unwrap_or_else(|| "None".into())
	);
}

pub fn format_swap_native_option(
	cid: &CommunityIdentifier,
	to: &AccountId,
	opt: &SwapNativeOption<Balance, Moment>,
) -> String {
	format!(
		"Issue SwapNativeOption for {cid} to {to}: allowance={}, rate={}, burn={}",
		opt.native_allowance,
		opt.rate.map(|r| format!("{r}")).unwrap_or_else(|| "oracle".into()),
		opt.do_burn
	)
}

pub fn format_swap_asset_option<AssetId: core::fmt::Debug>(
	cid: &CommunityIdentifier,
	to: &AccountId,
	opt: &SwapAssetOption<Balance, Moment, AssetId>,
) -> String {
	format!(
		"Issue SwapAssetOption for {cid} to {to}: asset={:?}, allowance={}, rate={}, burn={}",
		opt.asset_id,
		opt.asset_allowance,
		opt.rate.map(|r| format!("{r}")).unwrap_or_else(|| "oracle".into()),
		opt.do_burn
	)
}
