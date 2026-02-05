use crate::{
	cli_args::EncointerArgsExtractor,
	utils::{
		collective_propose_call, contains_sudo_pallet, ensure_payment, get_chain_api,
		get_councillors,
		keys::{get_accountid_from_str, get_pair_from_str},
		print_raw_call, send_and_wait_for_in_block, sudo_call, xt, OpaqueCall,
	},
};
use clap::ArgMatches;
use encointer_api_client_extension::{
	set_api_extrisic_params_builder, CommunitiesApi, EncointerXt, ParentchainExtrinsicSigner,
};
use encointer_node_notee_runtime::{AccountId, Balance};
use encointer_primitives::faucet::{Faucet, FaucetNameType, FromStr, WhiteListType};
use log::{error, info};
use parity_scale_codec::{Decode, Encode};
use sp_core::{crypto::Ss58Codec, sr25519 as sr25519_core, Pair};
use sp_keyring::Sr25519Keyring as AccountKeyring;
use substrate_api_client::{
	ac_compose_macros::{compose_call, compose_extrinsic},
	GetAccountInformation, GetStorage, SubmitAndWatch, XtStatus,
};

pub fn create_faucet(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();

		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));

		let faucet_name_raw = matches.faucet_name_arg().unwrap();
		let faucet_balance = matches.faucet_balance_arg().unwrap();
		let drip_amount = matches.faucet_drip_amount_arg().unwrap();

		let api2 = api.clone();
		let whitelist = if let Some(wl) = matches.whitelist_arg() {
			let whitelist_vec: Vec<_> = futures::future::join_all(wl.into_iter().map(|c| {
				let api_local = api2.clone();
				async move { api_local.verify_cid(c, None).await }
			}))
			.await;
			Some(WhiteListType::try_from(whitelist_vec).unwrap())
		} else {
			None
		};

		let faucet_name = FaucetNameType::from_str(faucet_name_raw).unwrap();
		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		let xt: EncointerXt<_> = compose_extrinsic!(
			api,
			"EncointerFaucet",
			"create_faucet",
			faucet_name,
			faucet_balance,
			whitelist,
			drip_amount
		)
		.unwrap();

		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;

		let result = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await;

		match result {
			Ok(report) => {
				for event in report.events.unwrap().iter() {
					if event.pallet_name() == "EncointerFaucet"
						&& event.variant_name() == "FaucetCreated"
					{
						println!(
							"{}",
							AccountId::decode(&mut event.field_bytes()[0..32].as_ref())
								.unwrap()
								.to_ss58check()
						);
					}
				}
			},
			Err(e) => {
				println!("[+] Couldn't execute the extrinsic due to {:?}\n", e);
			},
		};

		Ok(())
	})
	.into()
}
pub fn drip_faucet(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();

		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));

		let cid = api
			.verify_cid(matches.cid_arg().expect("please supply argument --cid"), None)
			.await;

		let cindex = matches.cindex_arg().unwrap();
		let faucet_account = get_accountid_from_str(matches.faucet_account_arg().unwrap());

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		let xt: EncointerXt<_> =
			compose_extrinsic!(api, "EncointerFaucet", "drip", faucet_account, cid, cindex)
				.unwrap();

		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;

		let result = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await;

		match result {
			Ok(_report) => {
				println!("Faucet dripped to {}", who.public());
			},
			Err(e) => {
				println!("[+] Couldn't execute the extrinsic due to {:?}\n", e);
			},
		};

		Ok(())
	})
	.into()
}
pub fn dissolve_faucet(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let signer = matches.signer_arg().map_or_else(
			|| AccountKeyring::Alice.pair(),
			|signer| get_pair_from_str(signer).into(),
		);
		let signer = ParentchainExtrinsicSigner::new(signer);

		let faucet_account = get_accountid_from_str(matches.faucet_account_arg().unwrap());
		let beneficiary = get_accountid_from_str(matches.faucet_beneficiary_arg().unwrap());

		let mut api = get_chain_api(matches).await;
		api.set_signer(signer);

		let dissolve_faucet_call = compose_call!(
			api.metadata(),
			"EncointerFaucet",
			"dissolve_faucet",
			faucet_account.clone(),
			beneficiary
		)
		.unwrap();

		// return calls as `OpaqueCall`s to get the same return type in both branches
		let dissolve_faucet_call = if contains_sudo_pallet(api.metadata()) {
			let dissolve_faucet_call = sudo_call(api.metadata(), dissolve_faucet_call);
			info!("Printing raw sudo call for js/apps:");
			print_raw_call("sudo(dissolve_faucet)", &dissolve_faucet_call);

			OpaqueCall::from_tuple(&dissolve_faucet_call)
		} else {
			let threshold = (get_councillors(&api).await.unwrap().len() / 2 + 1) as u32;
			info!("Printing raw collective propose calls with threshold {} for js/apps", threshold);
			let propose_dissolve_faucet =
				collective_propose_call(api.metadata(), threshold, dissolve_faucet_call);
			print_raw_call("collective_propose(dissolve_faucet)", &propose_dissolve_faucet);

			OpaqueCall::from_tuple(&propose_dissolve_faucet)
		};

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		send_and_wait_for_in_block(&api, xt(&api, dissolve_faucet_call).await, tx_payment_cid_arg)
			.await;

		println!("Faucet dissolved: {faucet_account:?}");
		Ok(())
	})
	.into()
}
pub fn close_faucet(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();

		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who)));

		let faucet_account = get_accountid_from_str(matches.faucet_account_arg().unwrap());

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		let xt: EncointerXt<_> =
			compose_extrinsic!(api, "EncointerFaucet", "close_faucet", faucet_account.clone())
				.unwrap();

		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		let report = api.submit_and_watch_extrinsic_until(xt, XtStatus::Ready).await.unwrap();

		println!("Faucet closed: {faucet_account}. status: '{:?}'", report.status);
		Ok(())
	})
	.into()
}
pub fn set_faucet_reserve_amount(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let signer = matches.signer_arg().map_or_else(
			|| AccountKeyring::Alice.pair(),
			|signer| get_pair_from_str(signer).into(),
		);
		let signer = ParentchainExtrinsicSigner::new(signer);

		let reserve_amount = matches.faucet_reserve_amount_arg().unwrap();

		let mut api = get_chain_api(matches).await;
		api.set_signer(signer);

		let set_reserve_amount_call =
			compose_call!(api.metadata(), "EncointerFaucet", "set_reserve_amount", reserve_amount)
				.unwrap();
		// return calls as `OpaqueCall`s to get the same return type in both branches
		let set_reserve_amount_call = if contains_sudo_pallet(api.metadata()) {
			let set_reserve_amount_call = sudo_call(api.metadata(), set_reserve_amount_call);
			info!("Printing raw sudo call for js/apps:");
			print_raw_call("sudo(set_reserve_amount)", &set_reserve_amount_call);

			OpaqueCall::from_tuple(&set_reserve_amount_call)
		} else {
			let threshold = (get_councillors(&api).await.unwrap().len() / 2 + 1) as u32;
			info!("Printing raw collective propose calls with threshold {} for js/apps", threshold);
			let propose_set_reserve_amount =
				collective_propose_call(api.metadata(), threshold, set_reserve_amount_call);
			print_raw_call("collective_propose(set_reserve_amount)", &propose_set_reserve_amount);

			OpaqueCall::from_tuple(&propose_set_reserve_amount)
		};

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		send_and_wait_for_in_block(
			&api,
			xt(&api, set_reserve_amount_call).await,
			tx_payment_cid_arg,
		)
		.await;

		println!("Reserve amount set: {reserve_amount:?}");
		Ok(())
	})
	.into()
}
pub fn list_faucets(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;

		let is_verbose = matches.verbose_flag();
		let maybe_at = matches.at_block_arg();

		let key_prefix =
			api.get_storage_map_key_prefix("EncointerFaucet", "Faucets").await.unwrap();

		let max_keys = 1000;
		let storage_keys = api
			.get_storage_keys_paged(Some(key_prefix), max_keys, None, maybe_at)
			.await
			.unwrap();

		if storage_keys.len() == max_keys as usize {
			error!("results can be wrong because max keys reached for query")
		}

		for storage_key in storage_keys.iter() {
			let key_postfix = storage_key.as_ref();
			let faucet_address =
				AccountId::decode(&mut key_postfix[key_postfix.len() - 32..].as_ref()).unwrap();
			let faucet: Faucet<AccountId, Balance> =
				api.get_storage_by_key(storage_key.clone(), maybe_at).await.unwrap().unwrap();

			if is_verbose {
				println!("address: {}", faucet_address.to_ss58check());
				println!("name: {}", String::from_utf8(faucet.name.to_vec()).unwrap());
				println!(
					"creator: {}",
					AccountId::decode(&mut faucet.creator.as_ref()).unwrap().to_ss58check()
				);
				println!(
					"balance: {}",
					api.get_account_data(&faucet_address).await.unwrap().unwrap().free
				);
				println!("drip amount: {}", faucet.drip_amount);
				if let Some(whitelist) = faucet.whitelist {
					println!("whitelist:");
					for cid in whitelist.to_vec() {
						println!("{}", cid);
					}
				} else {
					println!("whitelist: None");
				}
				println!("");
			} else {
				println! {"{}", faucet_address};
			}
		}
		Ok(())
	})
	.into()
}
