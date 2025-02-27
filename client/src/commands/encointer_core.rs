use crate::{
	cli_args::EncointerArgsExtractor,
	commands::frame::get_block_number,
	exit_code,
	utils::{
		ensure_payment, get_chain_api,
		keys::{get_accountid_from_str, get_pair_from_str},
	},
};
use clap::{value_t, ArgMatches};
use encointer_api_client_extension::{
	set_api_extrisic_params_builder, Api, CommunitiesApi, EncointerXt, ParentchainExtrinsicSigner,
};
use encointer_node_notee_runtime::{AccountId, BlockNumber, Hash, RuntimeEvent};
use encointer_primitives::balances::{to_U64F64, BalanceEntry, BalanceType, Demurrage};

use encointer_primitives::{communities::CommunityIdentifier, fixed::transcendental::exp};
use log::{debug, error, info};
use pallet_transaction_payment::FeeDetails;
use parity_scale_codec::Encode;
use sp_core::{crypto::Ss58Codec, sr25519 as sr25519_core, Pair};

use sp_rpc::number::NumberOrHex;
use std::str::FromStr;
use substrate_api_client::{
	ac_compose_macros::{compose_extrinsic, rpc_params},
	ac_primitives::Bytes,
	extrinsic::BalancesExtrinsics,
	rpc::Request,
	GetAccountInformation, GetStorage, SubmitAndWatch, SubscribeEvents, XtStatus,
};

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
				if maybe_at.is_some() {
					panic!("can't apply --at if --cid not set")
				};
				if matches.all_flag() {
					let community_balances = get_all_balances(&api, &accountid).await.unwrap();
					let bn = get_block_number(&api, maybe_at).await;
					for b in community_balances.iter() {
						let dr = get_demurrage_per_block(&api, b.0, maybe_at).await;
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
				let cid = api.verify_cid(cid_str, None).await;
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
				let cid = api.verify_cid(cid_str, None).await;
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

pub async fn get_community_balance(
	api: &Api,
	cid_str: &str,
	account_id: &AccountId,
	maybe_at: Option<Hash>,
) -> BalanceType {
	let cid = api.verify_cid(cid_str, maybe_at).await;
	let bn = get_block_number(api, maybe_at).await;
	let dr = get_demurrage_per_block(api, cid, maybe_at).await;

	if let Some(entry) = api
		.get_storage_double_map("EncointerBalances", "Balance", cid, account_id, maybe_at)
		.await
		.unwrap()
	{
		apply_demurrage(entry, bn, dr)
	} else {
		BalanceType::from_num(0)
	}
}

pub async fn get_community_issuance(
	api: &Api,
	cid_str: &str,
	maybe_at: Option<Hash>,
) -> BalanceType {
	let cid = api.verify_cid(cid_str, maybe_at).await;
	let bn = get_block_number(api, maybe_at).await;
	let dr = get_demurrage_per_block(api, cid, maybe_at).await;

	if let Some(entry) = api
		.get_storage_map("EncointerBalances", "TotalIssuance", cid, maybe_at)
		.await
		.unwrap()
	{
		apply_demurrage(entry, bn, dr)
	} else {
		BalanceType::from_num(0)
	}
}

async fn get_demurrage_per_block(
	api: &Api,
	cid: CommunityIdentifier,
	maybe_at: Option<Hash>,
) -> Demurrage {
	let d: Option<Demurrage> = api
		.get_storage_map("EncointerBalances", "DemurragePerBlock", cid, maybe_at)
		.await
		.unwrap();

	match d {
		Some(d) => {
			debug!("Fetched community specific demurrage per block {:?}", &d);
			d
		},
		None => {
			let d = api.get_constant("EncointerBalances", "DefaultDemurrage").await.unwrap();
			debug!("Fetched default demurrage per block {:?}", d);
			d
		},
	}
}

async fn get_all_balances(
	api: &Api,
	account_id: &AccountId,
) -> Option<Vec<(CommunityIdentifier, BalanceEntry<BlockNumber>)>> {
	api.client()
		.request("encointer_getAllBalances", rpc_params![account_id])
		.await
		.expect("Could not query all balances...")
}

pub async fn get_asset_fee_details(
	api: &Api,
	cid_str: &str,
	encoded_xt: &Bytes,
) -> Option<FeeDetails<NumberOrHex>> {
	let cid = api.verify_cid(cid_str, None).await;

	api.client()
		.request("encointer_queryAssetFeeDetails", rpc_params![cid, encoded_xt])
		.await
		.expect("Could not query asset fee details")
}
pub fn apply_demurrage(
	entry: BalanceEntry<BlockNumber>,
	current_block: BlockNumber,
	demurrage_per_block: Demurrage,
) -> BalanceType {
	let elapsed_time_block_number = current_block.checked_sub(entry.last_update).unwrap();
	let elapsed_time_u32: u32 = elapsed_time_block_number;
	let elapsed_time = Demurrage::from_num(elapsed_time_u32);
	let exponent = -demurrage_per_block * elapsed_time;
	debug!(
		"demurrage per block {}, current_block {}, last {}, elapsed_blocks {}",
		demurrage_per_block, current_block, entry.last_update, elapsed_time
	);
	let exp_result = exp(exponent).unwrap();
	entry.principal.checked_mul(to_U64F64(exp_result).unwrap()).unwrap()
}

async fn listen(matches: &ArgMatches<'_>) {
	let api = get_chain_api(matches).await;

	let block_count = value_t!(matches, "blocks", u32).ok();
	let event_count = value_t!(matches, "events", u32).ok();

	wait_for_blocks_or_events(&api, block_count, event_count).await;
}

pub async fn wait_for_blocks_or_events(
	api: &Api,
	target_block_count: Option<u32>,
	target_event_count: Option<u32>,
) {
	let mut subscription = api.subscribe_events().await.unwrap();
	let mut event_count = 0u32;
	let mut block_count = 0u32;
	loop {
		if target_event_count.is_some() && event_count >= target_event_count.unwrap() {
			return;
		};
		if target_block_count.is_some() && block_count > target_block_count.unwrap() {
			return;
		};

		let event_results = subscription.next_events::<RuntimeEvent, Hash>().await.unwrap();
		block_count += 1;

		match event_results {
			Ok(events) => {
				print_events(events, &mut event_count);
			},
			Err(_) => error!("couldn't decode event record list"),
		}
	}
}

pub fn print_events(
	events: Vec<substrate_api_client::ac_node_api::EventRecord<RuntimeEvent, Hash>>,
	encointer_event_count: &mut u32,
) {
	for evr in events {
		debug!("decoded: phase {:?} event {:?}", evr.phase, evr.event);
		match &evr.event {
			RuntimeEvent::EncointerCeremonies(ee) => {
				info!(">>>>>>>>>> ceremony event: {:?}", ee);
				*encointer_event_count += 1;
				match &ee {
					pallet_encointer_ceremonies::Event::ParticipantRegistered(
						cid,
						participant_type,
						accountid,
					) => {
						println!(
                            "Participant registered as {participant_type:?}, for cid: {cid:?}, account: {accountid}, "
                        );
					},
					_ => println!("Unsupported EncointerCommunities event"),
				}
			},
			RuntimeEvent::EncointerScheduler(ee) => {
				info!(">>>>>>>>>> scheduler event: {:?}", ee);
				*encointer_event_count += 1;
				match &ee {
					pallet_encointer_scheduler::Event::PhaseChangedTo(phase) => {
						println!("Phase changed to: {phase:?}");
					},
					pallet_encointer_scheduler::Event::CeremonySchedulePushedByOneDay => {
						println!("Ceremony schedule was pushed by one day");
					},
				}
			},
			RuntimeEvent::EncointerCommunities(ee) => {
				info!(">>>>>>>>>> community event: {:?}", ee);
				*encointer_event_count += 1;
				match &ee {
					pallet_encointer_communities::Event::CommunityRegistered(cid) => {
						println!("Community registered: cid: {cid:?}");
					},
					pallet_encointer_communities::Event::MetadataUpdated(cid) => {
						println!("Community metadata updated cid: {cid:?}");
					},
					pallet_encointer_communities::Event::NominalIncomeUpdated(cid, income) => {
						println!("Community metadata updated cid: {cid:?}, value: {income:?}");
					},
					pallet_encointer_communities::Event::DemurrageUpdated(cid, demurrage) => {
						println!("Community metadata updated cid: {cid:?}, value: {demurrage:?}");
					},
					_ => println!("Unsupported EncointerCommunities event"),
				}
			},
			RuntimeEvent::EncointerBalances(ee) => {
				*encointer_event_count += 1;
				println!(">>>>>>>>>> encointer balances event: {ee:?}");
			},
			RuntimeEvent::EncointerBazaar(ee) => {
				*encointer_event_count += 1;
				println!(">>>>>>>>>> encointer bazaar event: {ee:?}");
			},
			RuntimeEvent::System(ee) => match ee {
				frame_system::Event::ExtrinsicFailed { dispatch_error: _, dispatch_info: _ } => {
					error!("ExtrinsicFailed: {ee:?}");
				},
				frame_system::Event::ExtrinsicSuccess { dispatch_info } => {
					println!("ExtrinsicSuccess: {dispatch_info:?}");
				},
				_ => debug!("ignoring unsupported system Event"),
			},
			_ => debug!("ignoring unsupported module event: {:?}", evr.event),
		}
	}
}
