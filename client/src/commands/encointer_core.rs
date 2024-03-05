use crate::cli_args::EncointerArgsExtractor;
use crate::utils::keys::{get_accountid_from_str, get_pair_from_str};
use crate::utils::{
	collective_propose_call, contains_sudo_pallet, ensure_payment, get_councillors, print_raw_call,
	send_and_wait_for_in_block, sudo_call, xt, OpaqueCall,
};
use crate::{
	apply_demurrage, exit_code, get_all_balances, get_block_number, get_ceremony_index,
	get_chain_api, get_community_balance, get_community_issuance, get_demurrage_per_block, listen,
	set_api_extrisic_params_builder, verify_cid,
};
use clap::ArgMatches;
use encointer_api_client_extension::SchedulerApi;
use encointer_api_client_extension::{EncointerXt, ParentchainExtrinsicSigner};
use encointer_node_notee_runtime::Moment;
use encointer_primitives::balances::BalanceType;
use log::{debug, error, info};
use parity_scale_codec::{Encode};
use sp_core::{crypto::Ss58Codec, sr25519 as sr25519_core, Pair};
use sp_keyring::AccountKeyring;
use std::str::FromStr;
use substrate_api_client::ac_compose_macros::{compose_call, compose_extrinsic};
use substrate_api_client::extrinsic::BalancesExtrinsics;
use substrate_api_client::GetAccountInformation;
use substrate_api_client::SubmitAndWatch;
use substrate_api_client::XtStatus;

pub fn balance(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let account = matches.account_arg().unwrap();
		let maybe_at = matches.at_block_arg();
		let accountid = get_accountid_from_str(account);
		match matches.cid_arg() {
			Some(cid_str) => {
				let balance = get_community_balance(&api, cid_str, &accountid, maybe_at).await;
				println! {"{balance:?}"};
			},
			None => {
				if matches.all_flag() {
					let community_balances = get_all_balances(&api, &accountid).await.unwrap();
					let bn = get_block_number(&api, maybe_at).await;
					for b in community_balances.iter() {
						let dr = get_demurrage_per_block(&api, b.0).await;
						println!("{}: {}", b.0, apply_demurrage(b.1, bn, dr))
					}
				}
				let balance = if let Some(data) = api.get_account_data(&accountid).await.unwrap() {
					data.free
				} else {
					0
				};
				println!("{balance}");
			},
		};
		Ok(())
	})
	.into()
}
pub fn issuance(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let maybe_at = matches.at_block_arg();
		let cid_str = matches.cid_arg().expect("please supply argument --cid");
		let issuance = get_community_issuance(&api, cid_str, maybe_at).await;
		println! {"{issuance:?}"};
		Ok(())
	})
	.into()
}
pub fn transfer(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let mut api = get_chain_api(matches).await;
		let arg_from = matches.value_of("from").unwrap();
		let arg_to = matches.value_of("to").unwrap();
		if !matches.dryrun_flag() {
			let from = get_pair_from_str(arg_from);
			info!("from ss58 is {}", from.public().to_ss58check());
			let signer = ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(from));
			api.set_signer(signer);
		}
		let to = get_accountid_from_str(arg_to);
		info!("to ss58 is {}", to.to_ss58check());
		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		let tx_hash = match matches.cid_arg() {
			Some(cid_str) => {
				let cid = verify_cid(&api, cid_str, None).await;
				let amount = BalanceType::from_str(matches.value_of("amount").unwrap())
					.expect("amount can be converted to fixpoint");

				set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

				let xt: EncointerXt<_> = compose_extrinsic!(
					api,
					"EncointerBalances",
					"transfer",
					to.clone(),
					cid,
					amount
				)
				.unwrap();
				if matches.dryrun_flag() {
					println!("0x{}", hex::encode(xt.function.encode()));
					None
				} else {
					ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
					Some(api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await.unwrap())
				}
			},
			None => {
				let amount = matches
					.value_of("amount")
					.unwrap()
					.parse::<u128>()
					.expect("amount can be converted to u128");
				// todo: use keep_alive instead https://github.com/scs/substrate-api-client/issues/747
				let xt = api.balance_transfer_allow_death(to.clone().into(), amount).await.unwrap();
				if matches.dryrun_flag() {
					println!("0x{}", hex::encode(xt.function.encode()));
					None
				} else {
					ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
					Some(api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await.unwrap())
				}
			},
		};
		if let Some(txh) = tx_hash {
			info!("[+] Transaction included. Hash: {:?}\n", txh);
			let result = api.get_account_data(&to).await.unwrap().unwrap();
			println!("balance for {} is now {}", to, result.free);
		}
		Ok(())
	})
	.into()
}
pub fn transfer_all(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let mut api = get_chain_api(matches).await;
		let arg_from = matches.value_of("from").unwrap();
		let arg_to = matches.value_of("to").unwrap();
		let from = get_pair_from_str(arg_from);
		let to = get_accountid_from_str(arg_to);
		info!("from ss58 is {}", from.public().to_ss58check());
		info!("to ss58 is {}", to.to_ss58check());

		let signer = ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(from));
		api.set_signer(signer);
		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		let tx_hash = match matches.cid_arg() {
			Some(cid_str) => {
				let cid = verify_cid(&api, cid_str, None).await;
				set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

				let xt: EncointerXt<_> =
					compose_extrinsic!(api, "EncointerBalances", "transfer_all", to.clone(), cid)
						.unwrap();
				ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
				api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await.unwrap()
			},
			None => {
				error!("No cid specified");
				std::process::exit(exit_code::NO_CID_SPECIFIED);
			},
		};
		info!("[+] Transaction included. Hash: {:?}\n", tx_hash);
		let result = api.get_account_data(&to).await.unwrap().unwrap();
		println!("balance for {} is now {}", to, result.free);
		Ok(())
	})
	.into()
}
pub fn listen_to_events(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		listen(matches).await;
		Ok(())
	})
	.into()
}

pub fn get_phase(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;

		// >>>> add some debug info as well
		let bn = get_block_number(&api, None).await;
		debug!("block number: {}", bn);
		let cindex = get_ceremony_index(&api, None).await;
		info!("ceremony index: {}", cindex);
		let tnext: Moment = api.get_next_phase_timestamp().await.unwrap();
		debug!("next phase timestamp: {}", tnext);
		// <<<<

		let phase = api.get_current_phase().await.unwrap();
		println!("{phase:?}");
		Ok(())
	})
	.into()
}
pub fn next_phase(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let signer = matches.signer_arg().map_or_else(
			|| AccountKeyring::Alice.pair(),
			|signer| get_pair_from_str(signer).into(),
		);

		let mut api = get_chain_api(matches).await;
		let signer = ParentchainExtrinsicSigner::new(signer);
		api.set_signer(signer);
		let next_phase_call =
			compose_call!(api.metadata(), "EncointerScheduler", "next_phase").unwrap();

		// return calls as `OpaqueCall`s to get the same return type in both branches
		let next_phase_call = if contains_sudo_pallet(api.metadata()) {
			let sudo_next_phase_call = sudo_call(api.metadata(), next_phase_call);
			info!("Printing raw sudo call for js/apps:");
			print_raw_call("sudo(next_phase)", &sudo_next_phase_call);

			OpaqueCall::from_tuple(&sudo_next_phase_call)
		} else {
			let threshold = (get_councillors(&api).await.unwrap().len() / 2 + 1) as u32;
			info!("Printing raw collective propose calls with threshold {} for js/apps", threshold);
			let propose_next_phase =
				collective_propose_call(api.metadata(), threshold, next_phase_call);
			print_raw_call("collective_propose(next_phase)", &propose_next_phase);

			OpaqueCall::from_tuple(&propose_next_phase)
		};

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;

		send_and_wait_for_in_block(&api, xt(&api, next_phase_call).await, tx_payment_cid_arg).await;

		let phase = api.get_current_phase().await.unwrap();
		println!("Phase is now: {phase:?}");
		Ok(())
	})
	.into()
}
