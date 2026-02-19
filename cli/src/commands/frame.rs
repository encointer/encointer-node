use crate::{
	cli::Cli,
	utils::{ensure_payment, get_chain_api, keys::get_accountid_from_str},
	PREFUNDING_NR_OF_TRANSFER_EXTRINSICS,
};
use encointer_api_client_extension::{
	set_api_extrisic_params_builder, Api, EncointerXt, ExtrinsicAddress, ParentchainExtrinsicSigner,
};
use encointer_node_runtime::{AccountId, BlockNumber, Hash};
use log::{debug, info};
use parity_scale_codec::{Compact, Encode};
use sp_keyring::Sr25519Keyring as AccountKeyring;
use substrate_api_client::{
	ac_compose_macros::{compose_call, compose_extrinsic_offline},
	extrinsic::BalancesExtrinsics,
	GetBalance, GetChainInfo, GetTransactionPayment, SubmitAndWatch, XtStatus,
};

pub async fn print_metadata(cli: &Cli) {
	let api = get_chain_api(cli).await;
	println!("Metadata:\n {}", api.metadata().pretty_format().unwrap());
}

pub async fn fund(cli: &Cli, fundees: &[String]) {
	let mut api = get_chain_api(cli).await;
	api.set_signer(ParentchainExtrinsicSigner::new(AccountKeyring::Alice.pair()));

	let existential_deposit = api.get_existential_deposit().await.unwrap();
	info!("Existential deposit is = {:?}", existential_deposit);

	let mut nonce = api.get_nonce().await.unwrap();

	let amount = reasonable_native_balance(&api).await;

	let tx_payment_cid_arg = cli.tx_payment_cid.as_deref();
	set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

	for account in fundees.iter() {
		let to = get_accountid_from_str(account);
		let call = compose_call!(
			api.metadata(),
			"Balances",
			"transfer_keep_alive",
			ExtrinsicAddress::from(to.clone()),
			Compact(amount)
		)
		.unwrap();
		let xt: EncointerXt<_> = compose_extrinsic_offline!(
			api.clone().signer().unwrap(),
			call.clone(),
			api.extrinsic_params(nonce)
		);
		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		// send and watch extrinsic until ready
		println!("Alice-Faucet drips {amount} to {to} (Alice's nonce={nonce})");
		let _blockh = api.submit_and_watch_extrinsic_until(xt, XtStatus::Ready).await.unwrap();
		nonce += 1;
	}
}

pub async fn get_block_number(api: &Api, maybe_at: Option<Hash>) -> BlockNumber {
	let hdr = api.get_header(maybe_at).await.unwrap().unwrap();
	debug!("decoded: {:?}", hdr);
	hdr.number
}

async fn reasonable_native_balance(api: &Api) -> u128 {
	let alice: AccountId = AccountKeyring::Alice.into();
	let xt = api.balance_transfer_allow_death(alice.into(), 9999).await.unwrap();
	let fee = api
		.get_fee_details(&xt.encode().into(), None)
		.await
		.unwrap()
		.unwrap()
		.inclusion_fee
		.unwrap()
		.base_fee;
	let ed = api.get_existential_deposit().await.unwrap();
	// on the parachain we need this factor of 100 for some reason.
	ed + fee * PREFUNDING_NR_OF_TRANSFER_EXTRINSICS * 1000
}
