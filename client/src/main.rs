//  Copyright (c) 2019 Alain Brenzikofer
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//       http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

//! an RPC client to encointer node using websockets
//!
//! examples:
//! encointer-client-notee get-phase
//! encointer-client-notee transfer //Alice 5G9RtsTbiYJYQYMHbWfyPoeuuxNaCbC16tZ2JGrZ4gRKwz14 1000
//!

mod cli_args;
mod community_spec;
mod utils;

use crate::{
	community_spec::{
		add_location_call, new_community_call, read_community_spec_from_file, AddLocationCall,
		CommunitySpec,
	},
	utils::{
		batch_call, collective_propose_call, contains_sudo_pallet, ensure_payment, get_councillors,
		into_effective_cindex,
		keys::{get_accountid_from_str, get_pair_from_str, KEYSTORE_PATH, SR25519},
		print_raw_call, send_and_wait_for_in_block, sudo_call, xt, OpaqueCall,
	},
};
use clap::{value_t, AppSettings, Arg, ArgMatches};
use clap_nested::{Command, Commander};
use cli_args::{EncointerArgs, EncointerArgsExtractor};
use encointer_api_client_extension::{
	Api, AttestationState, CeremoniesApi, CommunitiesApi, CommunityCurrencyTip,
	CommunityCurrencyTipExtrinsicParamsBuilder, EncointerXt, ExtrinsicAddress,
	ParentchainExtrinsicSigner, SchedulerApi, ENCOINTER_CEREMONIES,
};
use encointer_node_notee_runtime::{
	AccountId, Balance, BalanceEntry, BalanceType, BlockNumber, Hash, Moment, RuntimeEvent,
	Signature, ONE_DAY,
};
use encointer_primitives::{
	balances::{to_U64F64, Demurrage},
	bazaar::{Business, BusinessIdentifier, OfferingData},
	ceremonies::{
		AttestationIndexType, ClaimOfAttendance, CommunityCeremony, CommunityReputation,
		MeetupIndexType, ParticipantIndexType, ProofOfAttendance, Reputation,
		ReputationLifetimeType,
	},
	communities::{CidName, CommunityIdentifier},
	democracy::{Proposal, ProposalAction, ProposalIdType, ReputationVec, Vote},
	faucet::{Faucet, FaucetNameType, FromStr as FaucetNameFromStr, WhiteListType},
	fixed::transcendental::exp,
	scheduler::{CeremonyIndexType, CeremonyPhaseType},
};
use futures::stream::{self, StreamExt, TryStreamExt};
use log::*;
use pallet_transaction_payment::FeeDetails;
use parity_scale_codec::{Compact, Decode, Encode};
use sp_application_crypto::{ed25519, sr25519};
use sp_core::{crypto::Ss58Codec, sr25519 as sr25519_core, ConstU32, Pair};
use sp_keyring::AccountKeyring;
use sp_keystore::Keystore;
use sp_rpc::number::NumberOrHex;
use sp_runtime::MultiSignature;
use std::{collections::HashMap, path::PathBuf, str::FromStr};
use substrate_api_client::{
	ac_compose_macros::{compose_call, compose_extrinsic, compose_extrinsic_offline, rpc_params},
	ac_primitives::{Bytes, SignExtrinsic},
	api::error::Error as ApiClientError,
	extrinsic::BalancesExtrinsics,
	rpc::{JsonrpseeClient, Request},
	GetAccountInformation, GetBalance, GetChainInfo, GetStorage, GetTransactionPayment,
	Result as ApiResult, SubmitAndWatch, SubscribeEvents, XtStatus,
};
use substrate_client_keystore::{KeystoreExt, LocalKeystore};

const PREFUNDING_NR_OF_TRANSFER_EXTRINSICS: u128 = 1000;
const VERSION: &str = env!("CARGO_PKG_VERSION");

mod exit_code {
	pub const WRONG_PHASE: i32 = 50;
	pub const FEE_PAYMENT_FAILED: i32 = 51;
	pub const INVALID_REPUTATION: i32 = 52;
	pub const RPC_ERROR: i32 = 60;
	pub const NO_CID_SPECIFIED: i32 = 70;
}

#[tokio::main]
async fn main() {
	env_logger::init();

	Commander::new()
		.options(|app| {
			app.arg(
				Arg::with_name("node-url")
					.short("u")
					.long("node-url")
					.global(true)
					.takes_value(true)
					.value_name("STRING")
					.default_value("ws://127.0.0.1")
					.help("node url"),
			)
			.arg(
				Arg::with_name("node-port")
					.short("p")
					.long("node-port")
					.global(true)
					.takes_value(true)
					.value_name("STRING")
					.default_value("9944")
					.help("node port"),
			)
			.optional_cid_arg()
			.tx_payment_cid_arg()
			.name("encointer-client-notee")
			.version(VERSION)
			.author("Encointer Association <info@encointer.org>")
			.about("interact with encointer-node-notee")
			.after_help("")
			.setting(AppSettings::ColoredHelp)
		})
		.args(|_args, _matches| "")
		// .add_cmd(
		//     Command::new("new-account")
		//         .description("Imports account into the key store. Either creates a new account or with the supplied seed.")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//                 .seed_arg()
		//         })
		//         .runner(|_args: &str, matches: &ArgMatches<'_>| {
		//
		//             let store = LocalKeystore::open(PathBuf::from(&KEYSTORE_PATH), None).unwrap();
		//
		//             // This does not place the key into the keystore if we have a seed, but it does
		//             // place it into the keystore if the seed is none.
		//             let key = store.sr25519_generate_new(
		//                 SR25519,
		//                 matches.seed_arg(),
		//             ).unwrap();
		//
		//             if let Some(suri) = matches.seed_arg() {
		//                 store.insert(SR25519, suri, &key.0).unwrap();
		//             }
		//
		//             drop(store);
		//             println!("{}", key.to_ss58check());
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("list-accounts")
		//         .description("lists all accounts in keystore")
		//         .runner(|_args: &str, _matches: &ArgMatches<'_>| {
		//             let store = LocalKeystore::open(PathBuf::from(&KEYSTORE_PATH), None).unwrap();
		//             info!("sr25519 keys:");
		//             for pubkey in store.public_keys::<sr25519::AppPublic>()
		//                 .unwrap()
		//                 .into_iter()
		//             {
		//                 println!("{}", pubkey.to_ss58check());
		//             }
		//             info!("ed25519 keys:");
		//             for pubkey in store.public_keys::<ed25519::AppPublic>()
		//                 .unwrap()
		//                 .into_iter()
		//             {
		//                 println!("{}", pubkey.to_ss58check());
		//             }
		//             drop(store);
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("print-metadata")
		//         .description("query node metadata and print it as json to stdout")
		//         .runner(move |_args: &str, matches: &ArgMatches<'_>| {
		//             let api = get_chain_api(matches).await;
		//             println!("Metadata:\n {}", api.metadata().pretty_format().unwrap());
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("faucet")
		//         .description("send some bootstrapping funds to supplied account(s)")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//             .fundees_arg()
		//         })
		//         .runner(move |_args: &str, matches: &ArgMatches<'_>| {
		//             let mut api = get_chain_api(matches).await;
		//             api
		//             .set_signer(ParentchainExtrinsicSigner::new(AccountKeyring::Alice.pair()));
		//             let accounts = matches.fundees_arg().unwrap();
		//
		//             let existential_deposit = api.get_existential_deposit().unwrap();
		//             info!("Existential deposit is = {:?}", existential_deposit);
		//
		//             let mut nonce = api.get_nonce().unwrap();
		//
		//             let amount = reasonable_native_balance(&api);
		//
		//             let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		//             set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);
		//
		//             for account in accounts.into_iter() {
		//                 let to = get_accountid_from_str(account);
		//                 let call = compose_call!(
		//                     api.metadata(),
		//                     "Balances",
		//                     "transfer_keep_alive",
		//                     ExtrinsicAddress::from(to.clone()),
		//                     Compact(amount)
		//                 ).unwrap();
		//                 let xt: EncointerXt<_> = compose_extrinsic_offline!(
		//                     api.clone().signer().unwrap(),
		//                     call.clone(),
		//                     api.extrinsic_params(nonce)
		//                 );
		//                 ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		//                 // send and watch extrinsic until ready
		//                 println!("Faucet drips {amount} to {to} (Alice's nonce={nonce})");
		//                 let _blockh = api
		//                     .submit_and_watch_extrinsic_until(xt, XtStatus::Ready).await
		//                     .unwrap();
		//                 nonce += 1;
		//             }
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("balance")
		//         .description("query on-chain balance for AccountId. If --cid is supplied, returns balance in that community. Otherwise balance of native ERT token")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//             .account_arg()
		//             .all_flag()
		//             .at_block_arg()
		//         })
		//         .runner(|_args: &str, matches: &ArgMatches<'_>| {
		//             let api = get_chain_api(matches).await;
		//             let account = matches.account_arg().unwrap();
		//             let maybe_at = matches.at_block_arg();
		//             let accountid = get_accountid_from_str(account);
		//             match matches.cid_arg() {
		//                 Some(cid_str) => {
		//                     let balance = get_community_balance(&api, cid_str, &accountid, maybe_at);
		//                     println!{"{balance:?}"};
		//                 }
		//                 None => {
		//                     if matches.all_flag() {
		//                         let community_balances = get_all_balances(&api, &accountid).unwrap();
		//                         let bn = get_block_number(&api, maybe_at);
		//                         for b in community_balances.iter() {
		//                             let dr = get_demurrage_per_block(&api, b.0);
		//                             println!("{}: {}", b.0, apply_demurrage(b.1, bn, dr))
		//                         }
		//                     }
		//                     let balance = if let Some(data) = api.get_account_data(&accountid).unwrap() {
		//                         data.free
		//                     } else {
		//                         0
		//                     };
		//                     println!("{balance}");
		//                 }
		//             };
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("issuance")
		//         .description("query total issuance for community. must supply --cid")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//             .at_block_arg()
		//         })
		//         .runner(|_args: &str, matches: &ArgMatches<'_>| {
		//             let api = get_chain_api(matches).await;
		//             let maybe_at = matches.at_block_arg();
		//             let cid_str = matches.cid_arg().expect("please supply argument --cid");
		//             let issuance = get_community_issuance(&api, cid_str, maybe_at);
		//             println!{"{issuance:?}"};
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("transfer")
		//         .description("transfer funds from one account to another. If --cid is supplied, send that community (amount is fixpoint). Otherwise send native ERT tokens (amount is integer)")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//             .dryrun_flag()
		//             .arg(
		//                 Arg::with_name("from")
		//                     .takes_value(true)
		//                     .required(true)
		//                     .value_name("SS58")
		//                     .help("sender's AccountId in ss58check format"),
		//             )
		//             .arg(
		//                 Arg::with_name("to")
		//                     .takes_value(true)
		//                     .required(true)
		//                     .value_name("SS58")
		//                     .help("recipient's AccountId in ss58check format"),
		//             )
		//             .arg(
		//                 Arg::with_name("amount")
		//                     .takes_value(true)
		//                     .required(true)
		//                     .value_name("U128")
		//                     .help("amount to be transferred"),
		//             )
		//         })
		//         .runner(|_args: &str, matches: &ArgMatches<'_>| {
		//             let mut api = get_chain_api(matches).await;
		//             let arg_from = matches.value_of("from").unwrap();
		//             let arg_to = matches.value_of("to").unwrap();
		//             if !matches.dryrun_flag() {
		//                 let from = get_pair_from_str(arg_from);
		//                 info!("from ss58 is {}", from.public().to_ss58check());
		//                 let signer = ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(from));
		//                 api.set_signer(signer);
		//             }
		//             let to = get_accountid_from_str(arg_to);
		//             info!("to ss58 is {}", to.to_ss58check());
		//             let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		//             let tx_hash = match matches.cid_arg() {
		//                 Some(cid_str) => {
		//                     let cid = verify_cid(&api, cid_str, None).await;
		//                     let amount = BalanceType::from_str(matches.value_of("amount").unwrap())
		//                         .expect("amount can be converted to fixpoint");
		//
		//                     set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);
		//
		//                     let xt: EncointerXt<_> = compose_extrinsic!(
		//                         api,
		//                         "EncointerBalances",
		//                         "transfer",
		//                         to.clone(),
		//                         cid,
		//                         amount
		//                     ).unwrap();
		//                     if matches.dryrun_flag() {
		//                         println!("0x{}", hex::encode(xt.function.encode()));
		//                         None
		//                     } else {
		//                         ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		//                         Some(api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await.unwrap())
		//                     }
		//                 },
		//                 None => {
		//                     let amount = matches.value_of("amount").unwrap().parse::<u128>()
		//                         .expect("amount can be converted to u128");
		//                     let xt = api.balance_transfer_allow_death(
		//                         to.clone().into(),
		//                         amount
		//                     );
		//                     if matches.dryrun_flag() {
		//                         println!("0x{}", hex::encode(xt.function.encode()));
		//                         None
		//                     } else {
		//                         ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		//                         Some(api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await.unwrap())
		//                     }
		//                 }
		//             };
		//             if let Some(txh) = tx_hash {
		//                 info!("[+] Transaction included. Hash: {:?}\n", txh);
		//                 let result = api.get_account_data(&to).unwrap().unwrap();
		//                 println!("balance for {} is now {}", to, result.free);
		//             }
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("transfer_all")
		//         .description("transfer all available funds from one account to another for a community specified with --cid.")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//             .arg(
		//                 Arg::with_name("from")
		//                     .takes_value(true)
		//                     .required(true)
		//                     .value_name("SS58")
		//                     .help("sender's AccountId in ss58check format"),
		//             )
		//             .arg(
		//                 Arg::with_name("to")
		//                     .takes_value(true)
		//                     .required(true)
		//                     .value_name("SS58")
		//                     .help("recipient's AccountId in ss58check format"),
		//             )
		//         })
		//         .runner(|_args: &str, matches: &ArgMatches<'_>| {
		//             let mut api = get_chain_api(matches).await;
		//             let arg_from = matches.value_of("from").unwrap();
		//             let arg_to = matches.value_of("to").unwrap();
		//             let from = get_pair_from_str(arg_from);
		//             let to = get_accountid_from_str(arg_to);
		//             info!("from ss58 is {}", from.public().to_ss58check());
		//             info!("to ss58 is {}", to.to_ss58check());
		//
		//             let signer = ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(from));
		//             api.set_signer(signer);
		//             let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		//             let tx_hash = match matches.cid_arg() {
		//                 Some(cid_str) => {
		//                     let cid = verify_cid(&api, cid_str, None).await;
		//                     set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);
		//
		//                     let xt: EncointerXt<_> = compose_extrinsic!(
		//                         api,
		//                         "EncointerBalances",
		//                         "transfer_all",
		//                         to.clone(),
		//                         cid
		//                     ).unwrap();
		//                     ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		//                     api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await.unwrap()
		//                 },
		//                 None => {
		//                     error!("No cid specified");
		//                     std::process::exit(exit_code::NO_CID_SPECIFIED);
		//                 }
		//             };
		//             info!("[+] Transaction included. Hash: {:?}\n", tx_hash);
		//             let result = api.get_account_data(&to).unwrap().unwrap();
		//             println!("balance for {} is now {}", to, result.free);
		//             Ok(())
		//
		//         }),
		// )
		// .add_cmd(
		//     Command::new("listen")
		//         .description("listen to on-chain events")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//             .arg(
		//                 Arg::with_name("events")
		//                     .short("e")
		//                     .long("await-events")
		//                     .takes_value(true)
		//                     .help("exit after given number of encointer events"),
		//             )
		//             .arg(
		//                 Arg::with_name("blocks")
		//                     .short("b")
		//                     .long("await-blocks")
		//                     .takes_value(true)
		//                     .help("exit after given number of blocks"),
		//             )
		//         })
		//         .runner(|_args: &str, matches: &ArgMatches<'_>| {
		//             listen(matches);
		//             Ok(())
		//         }),
		// )
		// // start encointer stuff
		// .add_cmd(
		//     Command::new("new-community")
		//         .description("Register new community")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//             .arg(
		//                 Arg::with_name("specfile")
		//                     .takes_value(true)
		//                     .required(true)
		//                     .help("enhanced geojson file that specifies a community"),
		//             )
		//             .signer_arg("account with necessary privileges")
		//         })
		//         .runner(|_args: &str, matches: &ArgMatches<'_>| {
		//             // -----setup
		//             let spec_file = matches.value_of("specfile").unwrap();
		//             let spec = read_community_spec_from_file(spec_file);
		//             let cid = spec.community_identifier();
		//
		//             let signer = matches.signer_arg()
		//                 .map_or_else(|| AccountKeyring::Alice.pair(), |signer| get_pair_from_str(signer).into());
		//             let signer = ParentchainExtrinsicSigner::new(signer);
		//
		//             let mut api = get_chain_api(matches).await;
		//             api.set_signer(signer);
		//
		//
		//             // ------- create calls for xt's
		//             let mut new_community_call = OpaqueCall::from_tuple(&new_community_call(&spec, api.metadata()));
		//             // only the first meetup location has been registered now. register all others one-by-one
		//             let add_location_calls = spec.locations().into_iter().skip(1).map(|l| add_location_call(api.metadata(), cid, l)).collect();
		//             let mut add_location_batch_call = OpaqueCall::from_tuple(&batch_call(api.metadata(), add_location_calls));
		//
		//
		//             if matches.signer_arg().is_none() {
		//                 // return calls as `OpaqueCall`s to get the same return type in both branches
		//                 (new_community_call, add_location_batch_call) = if contains_sudo_pallet(api.metadata()) {
		//                     let sudo_new_community = sudo_call(api.metadata(), new_community_call);
		//                     let sudo_add_location_batch = sudo_call(api.metadata(), add_location_batch_call);
		//                     info!("Printing raw sudo calls for js/apps for cid: {}", cid);
		//                     print_raw_call("sudo(new_community)", &sudo_new_community);
		//                     print_raw_call("sudo(utility_batch(add_location))", &sudo_add_location_batch);
		//
		//                     (OpaqueCall::from_tuple(&sudo_new_community), OpaqueCall::from_tuple(&sudo_add_location_batch))
		//
		//                 } else {
		//                     let threshold = (get_councillors(&api).await.unwrap().len() / 2 + 1) as u32;
		//                     info!("Printing raw collective propose calls with threshold {} for js/apps for cid: {}", threshold, cid);
		//                     let propose_new_community = collective_propose_call(api.metadata(), threshold, new_community_call);
		//                     let propose_add_location_batch = collective_propose_call(api.metadata(), threshold, add_location_batch_call);
		//                     print_raw_call("collective_propose(new_community)", &propose_new_community);
		//                     print_raw_call("collective_propose(utility_batch(add_location))", &propose_add_location_batch);
		//
		//                     (OpaqueCall::from_tuple(&propose_new_community), OpaqueCall::from_tuple(&propose_add_location_batch))
		//                 };
		//             }
		//
		//             // ---- send xt's to chain
		//             let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		//             set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);
		//
		//             send_and_wait_for_in_block(&api, xt(&api, new_community_call).await, matches.tx_payment_cid_arg());
		//             println!("{cid}");
		//
		//             if api.get_current_phase().await.unwrap() != CeremonyPhaseType::Registering {
		//                 error!("Wrong ceremony phase for registering new locations for {}", cid);
		//                 error!("Aborting without registering additional locations");
		//                 std::process::exit(exit_code::WRONG_PHASE);
		//             }
		//             send_and_wait_for_in_block(&api, xt(&api, add_location_batch_call).await, tx_payment_cid_arg);
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("add-locations")
		//         .description("Register new locations for a community")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//                 .signer_arg("account with necessary privileges")
		//                 .dryrun_flag()
		//                 .arg(
		//                     Arg::with_name("specfile")
		//                         .takes_value(true)
		//                         .required(true)
		//                         .help("geojson file that specifies locations to add as points"),
		//                 )
		//         })
		//         .runner(|_args: &str, matches: &ArgMatches<'_>| {
		//             // -----setup
		//             let spec_file = matches.value_of("specfile").unwrap();
		//             let spec = read_community_spec_from_file(spec_file);
		//
		//             let mut api = get_chain_api(matches).await;
		//             if !matches.dryrun_flag() {
		//                 let signer = matches.signer_arg()
		//                     .map_or_else(|| AccountKeyring::Alice.pair(), |signer| get_pair_from_str(signer).into());
		//                 info!("signer ss58 is {}", signer.public().to_ss58check());
		//                 let signer = ParentchainExtrinsicSigner::new(signer);
		//                 api.set_signer(signer);
		//             }
		//
		//             let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		//
		//             let cid = verify_cid(&api, matches.cid_arg().unwrap(), None).await;
		//
		//             let add_location_calls: Vec<AddLocationCall>= spec.locations().into_iter().map(|l|
		//                                                                           {
		//                                                                               info!("adding location {:?}", l);
		//                                                                               add_location_call(api.metadata(), cid, l)
		//                                                                           }
		//                 ).collect();
		//
		//             let mut add_location_maybe_batch_call = match  add_location_calls.as_slice() {
		//                 [call] => OpaqueCall::from_tuple(call),
		//                 _ => OpaqueCall::from_tuple(&batch_call(api.metadata(), add_location_calls.clone()))
		//             };
		//
		//             if matches.signer_arg().is_none() {
		//                 // return calls as `OpaqueCall`s to get the same return type in both branches
		//                 add_location_maybe_batch_call = if contains_sudo_pallet(api.metadata()) {
		//                     let sudo_add_location_batch = sudo_call(api.metadata(), add_location_maybe_batch_call);
		//                     info!("Printing raw sudo calls for js/apps for cid: {}", cid);
		//                     print_raw_call("sudo(utility_batch(add_location))", &sudo_add_location_batch);
		//                     OpaqueCall::from_tuple(&sudo_add_location_batch)
		//                 } else {
		//                     let threshold = (get_councillors(&api).await.unwrap().len() / 2 + 1) as u32;
		//                     info!("Printing raw collective propose calls with threshold {} for js/apps for cid: {}", threshold, cid);
		//                     let propose_add_location_batch = collective_propose_call(api.metadata(), threshold, add_location_maybe_batch_call);
		//                     print_raw_call("collective_propose(utility_batch(add_location))", &propose_add_location_batch);
		//                     OpaqueCall::from_tuple(&propose_add_location_batch)
		//                 };
		//             }
		//
		//             if matches.dryrun_flag() {
		//                 println!("0x{}", hex::encode(add_location_maybe_batch_call.encode()));
		//             } else {
		//                 // ---- send xt's to chain
		//                 if api.get_current_phase().await.unwrap() != CeremonyPhaseType::Registering {
		//                     error!("Wrong ceremony phase for registering new locations for {}", cid);
		//                     error!("Aborting without registering additional locations");
		//                     std::process::exit(exit_code::WRONG_PHASE);
		//                 }
		//                 send_and_wait_for_in_block(&api, xt(&api, add_location_maybe_batch_call).await, tx_payment_cid_arg);
		//             }
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("list-communities")
		//         .description("list all registered communities")
		//         .runner(|_args: &str, matches: &ArgMatches<'_>| {
		//             let api = get_chain_api(matches).await;
		//             let names = get_cid_names(&api).await.unwrap();
		//             println!("number of communities:  {}", names.len());
		//             for n in names.iter() {
		//                 let loc = api.get_locations(n.cid).await.unwrap();
		//                 println!("{}: {} locations: {}", n.cid, String::from_utf8(n.name.to_vec()).unwrap(), loc.len());
		//             }
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("list-locations")
		//         .description("list all meetup locations for a community")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//                 .at_block_arg()
		//         })
		//         .runner(|_args: &str, matches: &ArgMatches<'_>| {
		//             let api = get_chain_api(matches).await;
		//             let maybe_at = matches.at_block_arg();
		//             let cid = verify_cid(&api,
		//                 matches
		//                      .cid_arg()
		//                      .expect("please supply argument --cid"),
		//                 maybe_at
		//             ).await;
		//             println!("listing locations for cid {cid}");
		//             let loc = api.get_locations(cid).await.unwrap();
		//             for l in loc.iter() {
		//                 println!("lat: {} lon: {} (raw lat: {} lon: {})", l.lat, l.lon,
		//                          i128::decode(&mut l.lat.encode().as_slice()).unwrap(),
		//                          i128::decode(&mut l.lon.encode().as_slice()).unwrap()
		//                 );
		//             }
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("get-phase")
		//         .description("read current ceremony phase from chain")
		//         .runner(|_args: &str, matches: &ArgMatches<'_>| {
		//             let api = get_chain_api(matches).await;
		//
		//             // >>>> add some debug info as well
		//             let bn = get_block_number(&api, None).await;
		//             debug!("block number: {}", bn);
		//             let cindex = get_ceremony_index(&api, None).await;
		//             info!("ceremony index: {}", cindex);
		//             let tnext: Moment = api.get_next_phase_timestamp().await.unwrap();
		//             debug!("next phase timestamp: {}", tnext);
		//             // <<<<
		//
		//             let phase = api.get_current_phase().await.unwrap();
		//             println!("{phase:?}");
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("next-phase")
		//         .description("Advance ceremony state machine to next phase by ROOT call")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//                 .signer_arg("account with necessary privileges (sudo or councillor)")
		//         })
		//        .runner(move |_args: &str, matches: &ArgMatches<'_>| {
		//             let signer = matches.signer_arg()
		//                 .map_or_else(|| AccountKeyring::Alice.pair(), |signer| get_pair_from_str(signer).into());
		//
		//             let mut api = get_chain_api(matches).await;
		//             let signer = ParentchainExtrinsicSigner::new(signer);
		//             api.set_signer(signer);
		//             let next_phase_call = compose_call!(
		//                 api.metadata(),
		//                 "EncointerScheduler",
		//                 "next_phase"
		//             ).unwrap();
		//
		//             // return calls as `OpaqueCall`s to get the same return type in both branches
		//             let next_phase_call = if contains_sudo_pallet(api.metadata()) {
		//                 let sudo_next_phase_call = sudo_call(api.metadata(), next_phase_call);
		//                 info!("Printing raw sudo call for js/apps:");
		//                 print_raw_call("sudo(next_phase)", &sudo_next_phase_call);
		//
		//                 OpaqueCall::from_tuple(&sudo_next_phase_call)
		//
		//             } else {
		//                 let threshold = (get_councillors(&api).await.unwrap().len() / 2 + 1) as u32;
		//                 info!("Printing raw collective propose calls with threshold {} for js/apps", threshold);
		//                 let propose_next_phase = collective_propose_call(api.metadata(), threshold, next_phase_call).await;
		//                 print_raw_call("collective_propose(next_phase)", &propose_next_phase);
		//
		//                 OpaqueCall::from_tuple(&propose_next_phase)
		//             };
		//
		//             let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		//             set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);
		//
		//             send_and_wait_for_in_block(&api, xt(&api, next_phase_call).await, tx_payment_cid_arg);
		//
		//             let phase = api.get_current_phase().await.unwrap();
		//             println!("Phase is now: {phase:?}");
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("list-participants")
		//         .description("list all registered participants supplied community identifier and ceremony index")
		//         .options(|app| {
		//         app.setting(AppSettings::ColoredHelp)
		//             .ceremony_index_arg()
		//         })
		//         .runner(move |_args: &str, matches: &ArgMatches<'_>| {
		//             extract_and_execute(
		//                 matches, |api, cid| -> ApiResult<()>{
		//
		//                     let current_ceremony_index = get_ceremony_index(&api, None).await;
		//
		//                     let cindex = matches.ceremony_index_arg()
		//                         .map_or_else(|| current_ceremony_index , |ci| into_effective_cindex(ci, current_ceremony_index));
		//
		//                     println!("listing participants for cid {cid} and ceremony nr {cindex}");
		//
		//                     let counts = vec!["BootstrapperCount", "ReputableCount", "EndorseeCount", "NewbieCount"];
		//                     let count_query = |count_index| api.get_storage_map(ENCOINTER_CEREMONIES, counts[count_index], (cid, cindex), None).await;
		//
		//                     let registries = vec!["BootstrapperRegistry", "ReputableRegistry", "EndorseeRegistry", "NewbieRegistry"];
		//                     let account_query = |registry_index, p_index| api.get_storage_double_map(ENCOINTER_CEREMONIES, registries[registry_index],(cid, cindex), p_index, None).await;
		//
		//                     let mut num_participants: Vec<u64> = vec![0, 0, 0, 0];
		//                     for i in 0..registries.len() {
		//                         println!("Querying {}", registries[i]);
		//
		//                         let count: ParticipantIndexType = count_query(i)?.unwrap_or(0);
		//                         println!("number of participants assigned:  {count}");
		//                         num_participants[i] = count;
		//                         for p_index in 1..count +1 {
		//                             let accountid: AccountId = account_query(i, p_index)?.unwrap();
		//                             println!("{}[{}, {}] = {}", registries[i], cindex, p_index, accountid);
		//                         }
		//                     }
		//                     println!("total: {} guaranteed seats + {} newbies = {} total participants who would like to attend",
		//                              num_participants[0..=2].iter().sum::<u64>(),
		//                              num_participants[3],
		//                              num_participants[0..=3].iter().sum::<u64>()
		//                     );
		//                     Ok(())
		//                 }
		//             ).await.unwrap();
		//
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("list-meetups")
		//         .description("list all assigned meetups for supplied community identifier and ceremony index")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//                 .ceremony_index_arg()
		//         })
		//         .runner(move |_args: &str, matches: &ArgMatches<'_>| {
		//             extract_and_execute(
		//                 matches, |api, cid| -> ApiResult<()>{
		//
		//                     let current_ceremony_index = get_ceremony_index(&api, None).await;
		//
		//                     let cindex = matches.ceremony_index_arg()
		//                         .map_or_else(|| current_ceremony_index , |ci| into_effective_cindex(ci, current_ceremony_index));
		//
		//                     let community_ceremony = (cid, cindex);
		//
		//                     println!("listing meetups for cid {cid} and ceremony nr {cindex}");
		//
		//                     let stats = api.get_community_ceremony_stats(community_ceremony).await.unwrap();
		//
		//                     let mut num_assignees = 0u64;
		//
		//                     for meetup in stats.meetups.iter() {
		//                         println!("MeetupRegistry[{:?}, {}] location is {:?}, {:?}", &community_ceremony, meetup.index, meetup.location.lat, meetup.location.lon);
		//
		//                         println!("MeetupRegistry[{:?}, {}] meeting time is {:?}", &community_ceremony, meetup.index, meetup.time);
		//
		//                         if !meetup.registrations.is_empty() {
		//                             let num = meetup.registrations.len();
		//                             num_assignees += num as u64;
		//                             println!("MeetupRegistry[{:?}, {}] participants: {}", &community_ceremony, meetup.index, num);
		//                             for (participant, _registration) in meetup.registrations.iter() {
		//                                 println!("   {participant}");
		//                             }
		//                         } else {
		//                             println!("MeetupRegistry[{:?}, {}] EMPTY", &community_ceremony, meetup.index);
		//                         }
		//                     }
		//                     println!("total number of assignees: {num_assignees}");
		//                     Ok(())
		//                 }
		//             ).await.unwrap();
		//
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("print-ceremony-stats")
		//         .description("pretty prints all information for a community ceremony")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//                 .ceremony_index_arg()
		//         })
		//         .runner(move |_args: &str, matches: &ArgMatches<'_>| {
		//             extract_and_execute(
		//                 matches, |api, cid| -> ApiResult<()>{
		//
		//                     let current_ceremony_index = get_ceremony_index(&api, None).await;
		//
		//                     let cindex = matches.ceremony_index_arg()
		//                         .map_or_else(|| current_ceremony_index , |ci| into_effective_cindex(ci, current_ceremony_index));
		//
		//                     let community_ceremony = (cid, cindex);
		//
		//                     let stats = api.get_community_ceremony_stats(community_ceremony).await.unwrap();
		//
		//                     // serialization prints the the account id better than `debug`
		//                     println!("{}", serde_json::to_string_pretty(&stats).unwrap());
		//                     Ok(())
		//                 }
		//             ).unwrap();
		//
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("list-attestees")
		//         .description("list all attestees for participants for supplied community identifier and ceremony index")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//                 .ceremony_index_arg()
		//         })
		//         .runner(move |_args: &str, matches: &ArgMatches<'_>| {
		//             extract_and_execute(
		//                 matches, |api, cid| -> ApiResult<()>{
		//
		//                     let current_ceremony_index = get_ceremony_index(&api, None).await;
		//
		//                     let cindex = matches.ceremony_index_arg()
		//                         .map_or_else(|| current_ceremony_index , |ci| into_effective_cindex(ci, current_ceremony_index));
		//
		//                     println!("listing attestees for cid {cid} and ceremony nr {cindex}");
		//
		//                     let wcount = get_attestee_count(&api, (cid, cindex)).await;
		//                     println!("number of attestees:  {wcount}");
		//
		//                     println!("listing participants for cid {cid} and ceremony nr {cindex}");
		//
		//                     let counts = vec!["BootstrapperCount", "ReputableCount", "EndorseeCount", "NewbieCount"];
		//                     let count_query = |count_index| api.get_storage_map(ENCOINTER_CEREMONIES, counts[count_index], (cid, cindex), None).await;
		//
		//                     let registries = vec!["BootstrapperRegistry", "ReputableRegistry", "EndorseeRegistry", "NewbieRegistry"];
		//                     let account_query = |registry_index, p_index| async move {
		//                         api.get_storage_double_map(ENCOINTER_CEREMONIES, registries[registry_index],(cid, cindex), p_index, None).await
		//                     };
		//
		//                     let mut participants_windex = HashMap::new();
		//
		//                     for (i, item) in registries.iter().enumerate() {
		//                         println!("Querying {item}");
		//
		//                         let count: ParticipantIndexType = count_query(i)?.unwrap_or(0);
		//                         println!("number of participants assigned:  {count}");
		//
		//                         for p_index in 1..count +1 {
		//                             let accountid: AccountId = account_query(i, p_index)?.unwrap();
		//
		//                             match get_participant_attestation_index(&api, (cid, cindex), &accountid).await {
		//                                 Some(windex) => {
		//                                     participants_windex.insert(windex as AttestationIndexType, accountid)
		//                                 }
		//                                 _ => continue,
		//                             };
		//                         }
		//                     }
		//
		//                     let mut attestation_states = Vec::with_capacity(wcount as usize);
		//
		//                     for w in 1..wcount + 1 {
		//                         let attestor = participants_windex[&w].clone();
		//                         let meetup_index = api.get_meetup_index(&(cid, cindex), &attestor).await.unwrap().unwrap();
		//                         let attestees = api.get_attestees((cid, cindex), w).await.unwrap();
		//                         let vote = api.get_meetup_participant_count_vote((cid, cindex), attestor.clone()).await.unwrap();
		//                         let attestation_state = AttestationState::new(
		//                             (cid, cindex),
		//                             meetup_index,
		//                             vote,
		//                             w,
		//                             attestor,
		//                             attestees,
		//                         );
		//
		//                         attestation_states.push(attestation_state);
		//                     }
		//
		//                     // Group attestation states by meetup index
		//                     attestation_states.sort_by(|a, b| a.meetup_index.partial_cmp(&b.meetup_index).unwrap());
		//
		//                     for a in attestation_states.iter() {
		//                         println!("{a:?}");
		//                     }
		//
		//                     Ok(())
		//                 }
		//             ).await.unwrap();
		//
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("list-reputables")
		//         .description("list all reputables for all cycles within the current reputation-lifetime for all communities")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//                 .at_block_arg()
		//                 .verbose_flag()
		//         })
		//         .runner(move |_args: &str, matches: &ArgMatches<'_>| {
		//                     let api = get_chain_api(matches).await;
		//
		//                     let is_verbose = matches.verbose_flag();
		//                     let at_block = matches.at_block_arg();
		//
		//                     let lifetime = get_reputation_lifetime(&api, at_block).await;
		//                     let current_ceremony_index = get_ceremony_index(&api, at_block).await;
		//
		//
		//                     let first_ceremony_index_of_interest = current_ceremony_index.saturating_sub(lifetime);
		//                     let ceremony_indices: Vec<u32> = (first_ceremony_index_of_interest..current_ceremony_index).collect();
		//
		//                     let community_ids = get_cid_names(&api).await.unwrap().into_iter().map(|names| names.cid);
		//
		//                     let mut reputables_csv = Vec::new();
		//
		//                     println!("Listing the number of attested attendees for each community and ceremony for cycles [{:}:{:}]", ceremony_indices.first().unwrap(), ceremony_indices.last().unwrap());
		//                     for community_id in community_ids {
		//                         println!("Community ID: {community_id:?}");
		//                         let mut reputables: HashMap<AccountId, usize> = HashMap::new();
		//                         for ceremony_index in &ceremony_indices {
		//                             let (attendees, noshows) = get_attendees_for_community_ceremony(&api, (community_id, *ceremony_index), at_block).await;
		//                             println!("Cycle ID {ceremony_index:?}: Total attested attendees: {:} (noshows: {:})", attendees.len(), noshows.len());
		//                             for attendee in attendees {
		//                                 reputables_csv.push(format!("{community_id:?},{ceremony_index:?},{}", attendee.to_ss58check()));
		//                                 *reputables.entry(attendee.clone()).or_insert(0) += 1;
		//                             }
		//                         }
		//                         println!("Reputables in {community_id:?} (unique accounts with at least one attendance) {:}", reputables.keys().len());
		//                     }
		//                     if is_verbose {
		//                         for reputable in reputables_csv {
		//                             println!("{reputable}");
		//                         }
		//                     }
		//                     Ok(())
		//                 }),
		//         )
		// .add_cmd(
		//     Command::new("register-participant")
		//         .description("Register encointer ceremony participant for supplied community")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//             .account_arg()
		//             .signer_arg("Account which signs the tx.")
		//         })
		//         .runner(move |_args: &str, matches: &ArgMatches<'_>| {
		//             let arg_who = matches.account_arg().unwrap();
		//             let accountid = get_accountid_from_str(arg_who);
		//             let signer = match matches.signer_arg() {
		//                 Some(sig) => get_pair_from_str(sig),
		//                 None => get_pair_from_str(arg_who)
		//             };
		//
		//             let api = get_chain_api(matches).await;
		//             let cindex = get_ceremony_index(&api, None).await;
		//             let cid = verify_cid(&api,
		//                 matches
		//                     .cid_arg()
		//                     .expect("please supply argument --cid"),
		//                 None
		//             ).await;
		//             let rep = get_reputation(&api, &accountid, cid, cindex -1).await;
		//             info!("{} has reputation {:?}", accountid, rep);
		//             let proof = match rep {
		//                 Reputation::Unverified => None,
		//                 Reputation::UnverifiedReputable => None, // this should never be the case during Registering!
		//                 Reputation::VerifiedUnlinked => Some(prove_attendance(accountid, cid, cindex - 1, arg_who)),
		//                 Reputation::VerifiedLinked(_) => Some(prove_attendance(accountid, cid, cindex - 1, arg_who)),
		//             };
		//             debug!("proof: {:x?}", proof.encode());
		//             let current_phase = api.get_current_phase().await.unwrap();
		//             if !(current_phase == CeremonyPhaseType::Registering || current_phase == CeremonyPhaseType::Attesting) {
		//                 error!("wrong ceremony phase for registering participant");
		//                 std::process::exit(exit_code::WRONG_PHASE);
		//             }
		//             let mut api = api;
		//             let signer = ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(signer));
		//             api.set_signer(signer);
		//
		//             let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		//             set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);
		//
		//             let xt: EncointerXt<_> = compose_extrinsic!(
		//                 api,
		//                 "EncointerCeremonies",
		//                 "register_participant",
		//                 cid,
		//                 proof
		//             ).unwrap();
		//             ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		//             // send and watch extrinsic until ready
		//             let report = api.submit_and_watch_extrinsic_until(xt, XtStatus::Ready).await.unwrap();
		//             info!("Registration sent for {}. status: '{:?}'", arg_who, report.status);
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("upgrade-registration")
		//         .description("Upgrade registration to repuable for encointer ceremony participant for supplied community")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//             .account_arg()
		//             .signer_arg("Account which signs the tx.")
		//         })
		//         .runner(move |_args: &str, matches: &ArgMatches<'_>| {
		//             let arg_who = matches.account_arg().unwrap();
		//             let accountid = get_accountid_from_str(arg_who);
		//             let signer = match matches.signer_arg() {
		//                 Some(sig) => get_pair_from_str(sig),
		//                 None => get_pair_from_str(arg_who)
		//             };
		//
		//             let api = get_chain_api(matches).await;
		//             let cindex = get_ceremony_index(&api, None).await;
		//             let cid = verify_cid(&api,
		//                 matches
		//                     .cid_arg()
		//                     .expect("please supply argument --cid"),
		//                 None
		//             ).await;
		//
		//             let current_phase = api.get_current_phase().await.unwrap();
		//             if !(current_phase == CeremonyPhaseType::Registering || current_phase == CeremonyPhaseType::Attesting) {
		//                 error!("wrong ceremony phase for registering participant");
		//                 std::process::exit(exit_code::WRONG_PHASE);
		//             }
		//             let mut reputation_cindex = cindex;
		//             if current_phase == CeremonyPhaseType::Registering {
		//                 reputation_cindex -= 1;
		//             }
		//             let rep = get_reputation(&api, &accountid, cid, reputation_cindex).await;
		//             info!("{} has reputation {:?}", accountid, rep);
		//             let proof = match rep {
		//                 Reputation::VerifiedUnlinked => prove_attendance(accountid, cid, reputation_cindex, arg_who),
		//                 _ => {
		//                     error!("No valid reputation in last ceremony.");
		//                     std::process::exit(exit_code::INVALID_REPUTATION);
		//                 },
		//             };
		//
		//             let mut api = api;
		//             let signer = ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(signer));
		//             api.set_signer(signer);
		//
		//             let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		//             set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);
		//
		//             let xt: EncointerXt<_> = compose_extrinsic!(
		//                 api,
		//                 "EncointerCeremonies",
		//                 "upgrade_registration",
		//                 cid,
		//                 proof
		//             ).unwrap();
		//             ensure_payment(&api,  &xt.encode().into(), tx_payment_cid_arg).await;
		//             // send and watch extrinsic until ready
		//             let report = api.submit_and_watch_extrinsic_until(xt, XtStatus::Ready).await.unwrap();
		//             info!("Upgrade registration sent for {}. status: '{:?}'", arg_who, report.status);
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("unregister-participant")
		//         .description("Unregister encointer ceremony participant for supplied community")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//             .account_arg()
		//             .signer_arg("Account which signs the tx.")
		//             .ceremony_index_arg()
		//         })
		//         .runner(move |_args: &str, matches: &ArgMatches<'_>| {
		//             let arg_who = matches.account_arg().unwrap();
		//             let signer = match matches.signer_arg() {
		//                 Some(sig) => get_pair_from_str(sig),
		//                 None => get_pair_from_str(arg_who)
		//             };
		//
		//             let api = get_chain_api(matches).await;
		//
		//             let cid = verify_cid(&api,
		//                 matches
		//                     .cid_arg()
		//                     .expect("please supply argument --cid"),
		//                 None
		//             ).await;
		//
		//
		//             let cc = match matches.ceremony_index_arg() {
		//                 Some(cindex_arg) => {
		//                     let current_ceremony_index = get_ceremony_index(&api, None).await;
		//                     let cindex = into_effective_cindex(cindex_arg, current_ceremony_index);
		//                     Some((cid, cindex))
		//                 },
		//                 None => None,
		//              };
		//
		//             let current_phase = api.get_current_phase().await.unwrap();
		//             if !(current_phase == CeremonyPhaseType::Registering || current_phase == CeremonyPhaseType::Attesting) {
		//                 error!("wrong ceremony phase for unregistering");
		//                 std::process::exit(exit_code::WRONG_PHASE);
		//             }
		//             let mut api = api;
		//             let signer = ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(signer));
		//             api.set_signer(signer);
		//
		//             let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		//             set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);
		//
		//             let xt: EncointerXt<_> = compose_extrinsic!(
		//                 api,
		//                 "EncointerCeremonies",
		//                 "unregister_participant",
		//                 cid,
		//                 cc
		//             ).unwrap();
		//             ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		//             // Send and watch extrinsic until ready
		//             let report = api.submit_and_watch_extrinsic_until(xt, XtStatus::Ready).await.unwrap();
		//             info!("Unregister Participant sent for {}. status: '{:?}'", arg_who, report.status);
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("endorse-newcomers")
		//         .description("Endorse newbies with a bootstrapper account")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//                 .bootstrapper_arg()
		//                 .endorsees_arg()
		//         })
		//         .runner(move |_args: &str, matches: &ArgMatches<'_>| {
		//
		//             extract_and_execute(
		//                 matches, |mut api, cid| endorse_newcomers(&mut api, cid, matches)
		//             ).await.unwrap();
		//
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("get-bootstrappers-with-remaining-newbie-tickets")
		//         .description("Get the bootstrappers along with the remaining newbie tickets")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//         })
		//        .runner(move |_args: &str, matches: &ArgMatches<'_>| {
		//             let bs_with_tickets : Vec<BootstrapperWithTickets> = extract_and_execute(
		//                 matches, |api, cid| get_bootstrappers_with_remaining_newbie_tickets(&api, cid)
		//             ).await.unwrap();
		//
		//             info!("burned_bootstrapper_newbie_tickets = {:?}", bs_with_tickets);
		//
		//             // transform it to simple tuples, which is easier to parse in python
		//             let bt_vec = bs_with_tickets.into_iter()
		//                 .map(|bt| (bt.bootstrapper.to_ss58check(), bt.remaining_newbie_tickets)).collect::<Vec<_>>();
		//
		//             println!("{bt_vec:?}");
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("get-proof-of-attendance")
		//         .description("creates a proof of ProofOfAttendances for an <account> for the given ceremony index")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//                 .setting(AppSettings::AllowLeadingHyphen)
		//                 .account_arg()
		//                 .ceremony_index_arg()
		//         })
		//         .runner(move |_args: &str, matches: &ArgMatches<'_>| {
		//             let arg_who = matches.account_arg().unwrap();
		//             let accountid = get_accountid_from_str(arg_who);
		//             let api = get_chain_api(matches).await;
		//
		//             let current_ceremony_index = get_ceremony_index(&api, None).await;
		//
		//             let cindex_arg = matches.ceremony_index_arg().unwrap_or(-1);
		//             let cindex = into_effective_cindex(cindex_arg, current_ceremony_index);
		//
		//             let cid = verify_cid(
		//                 &api,
		//              matches.cid_arg().expect("please supply argument --cid"),
		//                 None
		//             ).await;
		//
		//             debug!("Getting proof for ceremony index: {:?}", cindex);
		//             let proof = prove_attendance(accountid, cid, cindex, arg_who);
		//             info!("Proof: {:?}\n", &proof);
		//             println!("0x{}", hex::encode(proof.encode()));
		//
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("attest-attendees")
		//         .description("Register encointer ceremony claim of attendances for supplied community")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//                 .account_arg()
		//                 .optional_cid_arg()
		//                 .attestees_arg()
		//         })
		//         .runner(move |_args: &str, matches: &ArgMatches<'_>| {
		//             let who = matches.account_arg().map(get_pair_from_str).unwrap();
		//
		//             let attestees: Vec<_> = matches.attestees_arg().unwrap()
		//                 .into_iter()
		//                 .map(get_accountid_from_str)
		//                 .collect();
		//
		//             let vote = attestees.len() as u32 + 1u32;
		//
		//             debug!("attestees: {:?}", attestees);
		//
		//             info!("send attest_attendees by {}", who.public());
		//
		//             let mut api =
		//              get_chain_api(matches).await;
		//              let signer = ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone()));
		//              api.set_signer(signer);
		//
		//             let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		//             set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);
		//
		//             let cid = verify_cid(&api,
		//                                  matches
		//                                      .cid_arg()
		//                                      .expect("please supply argument --cid"),
		//                                  None
		//             ).await;
		//
		//             let xt: EncointerXt<_> = compose_extrinsic!(
		//                 api,
		//                 "EncointerCeremonies",
		//                 "attest_attendees",
		//                 cid,
		//                 vote,
		//                 attestees
		//             ).unwrap();
		//             ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		//             let report = api.submit_and_watch_extrinsic_until(xt, XtStatus::Ready).await.unwrap();
		//
		//             println!("Claims sent by {}. status: '{:?}'", who.public(), report.status);
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("new-claim")
		//         .description("create a fresh claim of attendance for account")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//             .account_arg()
		//             .arg(
		//                 Arg::with_name("vote")
		//                     .takes_value(true)
		//                     .required(true)
		//                     .value_name("VOTE")
		//                     .help("participant's vote on the number of people present at meetup time"),
		//             )
		//         })
		//         .runner(move |_args: &str, matches: &ArgMatches<'_>| {
		//             extract_and_execute(
		//                 matches, |api, cid| -> ApiResult<()>{
		//                     let arg_who = matches.account_arg().unwrap();
		//                     let claimant = get_pair_from_str(arg_who);
		//
		//                     let n_participants = matches
		//                         .value_of("vote")
		//                         .unwrap()
		//                         .parse::<u32>()
		//                         .unwrap();
		//
		//                     let claim = new_claim_for(&api, &claimant.into(), cid, n_participants).await;
		//
		//                     println!("{}", hex::encode(claim));
		//                     Ok(())
		//                 }
		//             ).await.unwrap();
		//
		//             Ok(())
		//         }),
		// )
		// .add_cmd(
		//     Command::new("claim-reward")
		//         .description("Claim the rewards for all meetup participants of the last ceremony.")
		//         .options(|app| {
		//             app.setting(AppSettings::ColoredHelp)
		//                 .signer_arg("Account which signs the tx.")
		//                 .meetup_index_arg()
		//                 .all_flag()
		//         })
		//         .runner(move |_args: &str, matches: &ArgMatches<'_>| {
		//
		//             extract_and_execute(
		//                 matches, |api, cid| async move {
		//                     let signer = match matches.signer_arg() {
		//                         Some(sig) => get_pair_from_str(sig),
		//                         None => panic!("please specify --signer.")
		//                     };
		//                     let mut api = api;
		//                     let signer = ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(signer));
		//                     api.set_signer(signer.clone());
		//
		//                     let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		//                     let meetup_index_arg = matches.meetup_index_arg();
		//                     set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);
		//
		//                     if matches.all_flag() {
		//                         let mut cindex = get_ceremony_index(&api, None).await;
		//                         if api.get_current_phase().await.unwrap() == CeremonyPhaseType::Registering {
		//                             cindex -= 1;
		//                         }
		//                         let meetup_count = api
		//                         .get_storage_map("EncointerCeremonies", "MeetupCount", (cid, cindex), None).await
		//                         .unwrap().unwrap_or(0u64);
		//                         let calls: Vec<_> = (1u64..=meetup_count)
		//                         .map(|idx| compose_call!(
		//                             api.metadata(),
		//                             ENCOINTER_CEREMONIES,
		//                             "claim_rewards",
		//                             cid,
		//                             Option::<MeetupIndexType>::Some(idx)
		//                         ).unwrap())
		//                         .collect();
		//                         let batch_call = compose_call!(
		//                             api.metadata(),
		//                             "Utility",
		//                             "batch",
		//                             calls
		//                         ).unwrap();
		//                         send_and_wait_for_in_block(&api, xt(&api, batch_call).await, tx_payment_cid_arg);
		//                         println!("Claiming reward for all meetup indexes. xt-status: 'ready'");
		//                     } else {
		//                         let meetup_index = meetup_index_arg;
		//                         let xt: EncointerXt<_> = compose_extrinsic!(
		//                             api,
		//                             ENCOINTER_CEREMONIES,
		//                             "claim_rewards",
		//                             cid,
		//                             meetup_index
		//                         ).unwrap();
		//                         ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		//                         let report = api.submit_and_watch_extrinsic_until(xt, XtStatus::Ready).await.unwrap();
		//                         match meetup_index_arg {
		//                             Some(idx)=>{println!("Claiming reward for meetup_index {idx}. xt-status: '{:?}'", report.status);}
		//                             None=>{println!("Claiming reward for {}. xt-status: 'ready'", signer.public_account_id());}
		//                         }
		//                     }
		//                 }
		//             );
		//
		//             Ok(())
		//         }),
		// )
		.add_cmd(
		    Command::new("reputation")
		        .description("List reputation history for an account")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()})
		        .runner(cmd_reputation),
		)
		.add_cmd(
		    Command::new("create-business")
		        .description("Register a community business on behalf of the account")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		                .ipfs_cid_arg()
		        })
		        .runner(cmd_create_business),
		)
		.add_cmd(
		    Command::new("update-business")
		        .description("Update an already existing community business on behalf of the account")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		                .ipfs_cid_arg()
		        })
		        .runner(cmd_update_business),
		)
		.add_cmd(
		    Command::new("create-offering")
		        .description("Create an offering for the business belonging to account")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		                .ipfs_cid_arg()
		        })
		        .runner(cmd_create_offering),
		)
		.add_cmd(
		    Command::new("list-businesses")
		        .description("List businesses for a community")
		        .runner(cmd_list_businesses),
		)
		.add_cmd(
		    Command::new("list-offerings")
		        .description("List offerings for a community")
		        .runner(cmd_list_offerings),
		)
		.add_cmd(
		    Command::new("list-business-offerings")
		        .description("List offerings for a business")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		        })
		        .runner(cmd_list_business_offerings),
		)
		.add_cmd(
		        Command::new("purge-community-ceremony")
		            .description("purge all history within the provided ceremony index range for the specified community")
		            .options(|app| {
		                app.setting(AppSettings::ColoredHelp)
		                    .from_cindex_arg()
		                    .to_cindex_arg()

		            })
		        .runner(cmd_purge_community_ceremony),
		)
		.add_cmd(
		    Command::new("set-meetup-time-offset")
		        .description("signed value to offset the ceremony meetup time relative to solar noon")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .setting(AppSettings::AllowLeadingHyphen)
		                .time_offset_arg()
		        })
		       .runner(cmd_set_meetup_time_offset),
		)
		.add_cmd(
		    Command::new("create-faucet")
		        .description("Create faucet")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		                .faucet_name_arg()
		                .faucet_balance_arg()
		                .faucet_drip_amount_arg()
		                .whitelist_arg()
		        })
		        .runner(cmd_create_faucet),
		)
		.add_cmd(
		    Command::new("drip-faucet")
		        .description("Drip faucet. args: 1. faucet account, 2. cindex of the reputation. use --cid to specify the community.")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		                .faucet_account_arg()
		                .cindex_arg()
		        })
		        .runner(cmd_drip_faucet),
		)
		.add_cmd(
		    Command::new("dissolve-faucet")
		        .description("can only be called by root. args: 1. faucet address, 2. beneficiary of the remaining funds.")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .signer_arg("account with necessary privileges (sudo or councillor)")
		                .faucet_account_arg()
		                .faucet_beneficiary_arg()
		        })
		       .runner(cmd_dissolve_faucet),
		)
		.add_cmd(
		    Command::new("close-faucet")
		        .description("lazy garbage collection. can only be called by faucet creator and only once the faucet is empty")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		                .faucet_account_arg()
		        })
		        .runner(cmd_close_faucet),
		)
		.add_cmd(
		    Command::new("set-faucet-reserve-amount")
		        .description("Set faucet pallet reserve amount")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .signer_arg("account with necessary privileges (sudo or councillor)")
		                .faucet_reserve_amount_arg()
		        })
		       .runner(cmd_set_faucet_reserve_amount),
		)
		.add_cmd(
		    Command::new("list-faucets")
		        .description("list all faucets. use -v to get faucet details.")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .at_block_arg()
		                .verbose_flag()
		        })
		       .runner(cmd_list_faucets)
		)
		.add_cmd(
			Command::new("submit-set-inactivity-timeout-proposal")
				.description("Submit set inactivity timeout proposal")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.account_arg()
						.inactivity_timeout_arg()
				})
				.runner(cmd_submit_set_inactivity_timeout_proposal),
		)
		.add_cmd(
			Command::new("list-proposals")
				.description("list all proposals.")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.at_block_arg()
				})
			   .runner(cmd_list_proposals),
				)
		.add_cmd(
			Command::new("vote")
				.description("Submit vote for porposal. Vote is either ay or nay. Reputation vec to be specified as cid1_cindex1,cid2_cindex2,...")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.account_arg()
						.proposal_id_arg()
						.vote_arg()
						.reputation_vec_arg()
				})
				.runner(cmd_vote),
		)
		.add_cmd(
			Command::new("update-proposal-state")
				.description("Update proposal state")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp).account_arg().proposal_id_arg()
				})
				.runner(cmd_update_proposal_state),
		)
		// To handle when no subcommands match
		.no_cmd(|_args, _matches| {
			println!("No subcommand matched");
			Ok(())
		})
		.run();
}

fn cmd_reputation(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let account = matches.account_arg().unwrap();
		let account_id = get_accountid_from_str(account);
		if let Some(reputation) = get_reputation_history(&api, &account_id).await {
			for rep in reputation.iter() {
				println!("{}, {}, {:?}", rep.0, rep.1.community_identifier, rep.1.reputation);
			}
		} else {
			error!("could not fetch reputation over rpc");
			std::process::exit(exit_code::RPC_ERROR);
		}
		Ok(())
	})
	.into()
}
fn cmd_create_business(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		send_bazaar_xt(matches, &BazaarCalls::CreateBusiness).await.unwrap();
		Ok(())
	})
	.into()
}
fn cmd_update_business(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		send_bazaar_xt(matches, &BazaarCalls::UpdateBusiness).await.unwrap();
		Ok(())
	})
	.into()
}
fn cmd_create_offering(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		send_bazaar_xt(matches, &BazaarCalls::CreateOffering).await.unwrap();
		Ok(())
	})
	.into()
}
fn cmd_list_businesses(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
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
fn cmd_list_offerings(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
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

fn cmd_list_business_offerings(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
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
fn cmd_set_meetup_time_offset(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let mut api = get_chain_api(matches).await;
		let signer = ParentchainExtrinsicSigner::new(AccountKeyring::Alice.pair());
		api.set_signer(signer);
		let time_offset = matches.time_offset_arg().unwrap_or(0);
		let call = compose_call!(
			api.metadata(),
			"EncointerCeremonies",
			"set_meetup_time_offset",
			time_offset
		)
		.unwrap();

		// return calls as `OpaqueCall`s to get the same return type in both branches
		let privileged_call = if contains_sudo_pallet(api.metadata()) {
			let sudo_call = sudo_call(api.metadata(), call);
			info!("Printing raw sudo call for js/apps:");
			print_raw_call("sudo(...)", &sudo_call);
			OpaqueCall::from_tuple(&sudo_call)
		} else {
			let threshold = (get_councillors(&api).await.unwrap().len() / 2 + 1) as u32;
			info!("Printing raw collective propose calls with threshold {} for js/apps", threshold);
			let propose_call = collective_propose_call(api.metadata(), threshold, call);
			print_raw_call("collective_propose(...)", &propose_call);
			OpaqueCall::from_tuple(&propose_call)
		};

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);
		let xt = xt(&api, privileged_call).await;
		send_and_wait_for_in_block(&api, xt, tx_payment_cid_arg);
		Ok(())
	})
	.into()
}

fn cmd_purge_community_ceremony(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let sudoer = AccountKeyring::Alice.pair();
		let signer = ParentchainExtrinsicSigner::new(sudoer);
		let mut api = get_chain_api(matches).await;
		api.set_signer(signer);

		let current_ceremony_index = get_ceremony_index(&api, None).await;

		let from_cindex_arg = matches.from_cindex_arg().unwrap_or(0);
		let to_cindex_arg = matches.to_cindex_arg().unwrap_or(0);

		let from_cindex = into_effective_cindex(from_cindex_arg, current_ceremony_index);
		let to_cindex = into_effective_cindex(to_cindex_arg, current_ceremony_index);

		if from_cindex > to_cindex {
			panic!("'from' <= 'to' ceremony index violated");
		}
		let cid =
			verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;
		println!("purging ceremony index range [{from_cindex}  {to_cindex}] for community {cid}");

		let calls: Vec<_> = (from_cindex..=to_cindex)
			.map(|idx| {
				compose_call!(
					api.metadata(),
					"EncointerCeremonies",
					"purge_community_ceremony",
					(cid, idx)
				)
				.unwrap()
			})
			.collect();
		let batch_call = compose_call!(api.metadata(), "Utility", "batch", calls).unwrap();
		let unsigned_sudo_call =
			compose_call!(api.metadata(), "Sudo", "sudo", batch_call.clone()).unwrap();
		info!(
			"raw sudo batch call to sign with js/apps {}: 0x{}",
			cid,
			hex::encode(unsigned_sudo_call.encode())
		);

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);
		let xt: EncointerXt<_> = compose_extrinsic!(api, "Sudo", "sudo", batch_call).unwrap();
		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		let tx_report = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await.unwrap();
		info!("[+] Transaction got included. Block Hash: {:?}\n", tx_report.block_hash.unwrap());
		Ok(())
	})
	.into()
}
fn cmd_create_faucet(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();

		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));

		let faucet_name_raw = matches.faucet_name_arg().unwrap();
		let faucet_balance = matches.faucet_balance_arg().unwrap();
		let drip_amount = matches.faucet_drip_amount_arg().unwrap();

		let api2 = api.clone();
		let whitelist = futures::future::join_all(matches.whitelist_arg().map(|wl| async move {
			let whitelist_vec: Vec<_> = futures::future::join_all(wl.into_iter().map(|c| {
				let api_local = api2.clone();
				async move { verify_cid(&api_local, c, None).await }
			}))
			.await;
			WhiteListType::try_from(whitelist_vec).unwrap()
		}))
		.await;

		let faucet_name = FaucetNameType::from_str(faucet_name_raw).unwrap();
		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);

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
fn cmd_drip_faucet(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();

		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));

		let cid =
			verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;

		let cindex = matches.cindex_arg().unwrap();
		let faucet_account = get_accountid_from_str(matches.faucet_account_arg().unwrap());

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);

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
fn cmd_dissolve_faucet(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
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
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);

		send_and_wait_for_in_block(&api, xt(&api, dissolve_faucet_call).await, tx_payment_cid_arg);

		println!("Faucet dissolved: {faucet_account:?}");
		Ok(())
	})
	.into()
}
fn cmd_close_faucet(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();

		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who)));

		let faucet_account = get_accountid_from_str(matches.faucet_account_arg().unwrap());

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);

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
fn cmd_set_faucet_reserve_amount(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
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
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);

		send_and_wait_for_in_block(
			&api,
			xt(&api, set_reserve_amount_call).await,
			tx_payment_cid_arg,
		);

		println!("Reserve amount set: {reserve_amount:?}");
		Ok(())
	})
	.into()
}
fn cmd_list_faucets(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;

		let is_verbose = matches.verbose_flag();
		let at_block = matches.at_block_arg();

		let key_prefix =
			api.get_storage_map_key_prefix("EncointerFaucet", "Faucets").await.unwrap();

		let max_keys = 1000;
		let storage_keys = api
			.get_storage_keys_paged(Some(key_prefix), max_keys, None, at_block)
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
				api.get_storage_by_key(storage_key.clone(), at_block).await.unwrap().unwrap();

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
fn cmd_submit_set_inactivity_timeout_proposal(
	_args: &str,
	matches: &ArgMatches<'_>,
) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();
		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));
		let inactivity_timeout = matches.inactivity_timeout_arg().unwrap();
		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);

		let xt: EncointerXt<_> = compose_extrinsic!(
			api,
			"EncointerDemocracy",
			"submit_proposal",
			ProposalAction::SetInactivityTimeout(inactivity_timeout)
		)
		.unwrap();
		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		let _result = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await;
		println!("Proposal Submitted: Set inactivity timeout to {inactivity_timeout:?}");
		Ok(())
	})
	.into()
}
fn cmd_list_proposals(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;
		let at_block = matches.at_block_arg();
		let key_prefix =
			api.get_storage_map_key_prefix("EncointerDemocracy", "Proposals").await.unwrap();
		let max_keys = 1000;
		let storage_keys = api
			.get_storage_keys_paged(Some(key_prefix), max_keys, None, at_block)
			.await
			.unwrap();
		if storage_keys.len() == max_keys as usize {
			error!("results can be wrong because max keys reached for query")
		}
		for storage_key in storage_keys.iter() {
			let key_postfix = storage_key.as_ref();
			let proposal_id =
				ProposalIdType::decode(&mut key_postfix[key_postfix.len() - 16..].as_ref())
					.unwrap();
			let proposal: Proposal<BlockNumber> =
				api.get_storage_by_key(storage_key.clone(), at_block).await.unwrap().unwrap();
			println!("id: {}", proposal_id);
			println!("action: {:?}", proposal.action);
			println!("start block: {}", proposal.start);
			println!("start cindex: {}", proposal.start_cindex);
			println!("state: {:?}", proposal.state);
			println!("");
		}
		Ok(())
	})
	.into()
}
fn cmd_vote(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();
		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));
		let proposal_id = matches.proposal_id_arg().unwrap();
		let vote_raw = matches.vote_arg().unwrap();
		let vote = match vote_raw {
			"aye" => Vote::Aye,
			"nay" => Vote::Nay,
			&_ => panic!("invalid vote"),
		};
		let reputation_vec: Vec<CommunityCeremony> = futures::future::join_all(
			matches
				.reputation_vec_arg()
				.ok_or(clap::Error::with_description(
					"missing reputation-vec argument",
					clap::ErrorKind::MissingRequiredArgument,
				))?
				.into_iter()
				.map(|rep| {
					let api_local = api.clone();
					async move {
						let cc: Vec<_> = rep.split("_").collect();
						(
							verify_cid(&api_local, cc[0], None).await,
							cc[1].parse::<CeremonyIndexType>().unwrap(),
						)
					}
				}),
		)
		.await;
		let reputation_bvec = ReputationVec::<ConstU32<1024>>::try_from(reputation_vec);

		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);
		let xt: EncointerXt<_> = compose_extrinsic!(
			api,
			"EncointerDemocracy",
			"vote",
			proposal_id,
			vote,
			reputation_bvec
		)
		.unwrap();
		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		let _result = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await;
		println!("Vote submitted: {vote_raw:?} for proposal {proposal_id:?}");
		Ok(())
	})
	.into()
}
fn cmd_update_proposal_state(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let who = matches.account_arg().map(get_pair_from_str).unwrap();
		let mut api = get_chain_api(matches).await;
		api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(who.clone())));
		let proposal_id = matches.proposal_id_arg().unwrap();
		let tx_payment_cid_arg = matches.tx_payment_cid_arg();
		set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg).await;
		let xt: EncointerXt<_> =
			compose_extrinsic!(api, "EncointerDemocracy", "update_proposal_state", proposal_id)
				.unwrap();
		ensure_payment(&api, &xt.encode().into(), tx_payment_cid_arg).await;
		let _result = api.submit_and_watch_extrinsic_until(xt, XtStatus::InBlock).await;
		println!("Proposal state updated for proposal {proposal_id:?}");
		Ok(())
	})
	.into()
}
async fn get_chain_api(matches: &ArgMatches<'_>) -> Api {
	let url = format!(
		"{}:{}",
		matches.value_of("node-url").unwrap(),
		matches.value_of("node-port").unwrap()
	);
	debug!("connecting to {}", url);
	let client = JsonrpseeClient::new(&url).await.expect("node URL is incorrect");
	Api::new(client).await.unwrap()
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
	ed + fee * PREFUNDING_NR_OF_TRANSFER_EXTRINSICS
}

async fn listen(matches: &ArgMatches<'_>) {
	let api = get_chain_api(matches).await;
	debug!("Subscribing to events");
	let mut subscription = api.subscribe_events().await.unwrap();
	let mut count = 0u32;
	let mut blocks = 0u32;
	loop {
		if matches.is_present("events")
			&& count >= value_t!(matches.value_of("events"), u32).unwrap()
		{
			return;
		};
		if matches.is_present("blocks")
			&& blocks > value_t!(matches.value_of("blocks"), u32).unwrap()
		{
			return;
		};
		let event_results = subscription.next_events::<RuntimeEvent, Hash>().await.unwrap();
		blocks += 1;
		match event_results {
			Ok(evts) => {
				for evr in evts {
					debug!("decoded: phase {:?} event {:?}", evr.phase, evr.event);
					match &evr.event {
						RuntimeEvent::EncointerCeremonies(ee) => {
							count += 1;
							info!(">>>>>>>>>> ceremony event: {:?}", ee);
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
							count += 1;
							info!(">>>>>>>>>> scheduler event: {:?}", ee);
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
							count += 1;
							info!(">>>>>>>>>> community event: {:?}", ee);
							match &ee {
								pallet_encointer_communities::Event::CommunityRegistered(cid) => {
									println!("Community registered: cid: {cid:?}");
								},
								pallet_encointer_communities::Event::MetadataUpdated(cid) => {
									println!("Community metadata updated cid: {cid:?}");
								},
								pallet_encointer_communities::Event::NominalIncomeUpdated(
									cid,
									income,
								) => {
									println!(
										"Community metadata updated cid: {cid:?}, value: {income:?}"
									);
								},
								pallet_encointer_communities::Event::DemurrageUpdated(
									cid,
									demurrage,
								) => {
									println!(
										"Community metadata updated cid: {cid:?}, value: {demurrage:?}"
									);
								},
								_ => println!("Unsupported EncointerCommunities event"),
							}
						},
						RuntimeEvent::EncointerBalances(ee) => {
							count += 1;
							println!(">>>>>>>>>> encointer balances event: {ee:?}");
						},
						RuntimeEvent::EncointerBazaar(ee) => {
							count += 1;
							println!(">>>>>>>>>> encointer bazaar event: {ee:?}");
						},
						RuntimeEvent::System(ee) => match ee {
							frame_system::Event::ExtrinsicFailed {
								dispatch_error: _,
								dispatch_info: _,
							} => {
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
			},
			Err(_) => error!("couldn't decode event record list"),
		}
	}
}

/// Extracts api and cid from `matches` and execute the given `closure` with them.
async fn extract_and_execute<T>(
	matches: &ArgMatches<'_>,
	closure: impl FnOnce(Api, CommunityIdentifier) -> T,
) -> T {
	let api = get_chain_api(matches).await;
	let cid =
		verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;
	closure(api, cid)
}

async fn verify_cid(api: &Api, cid: &str, maybe_at: Option<Hash>) -> CommunityIdentifier {
	let cids = get_community_identifiers(api, maybe_at).await.expect("no community registered");
	let cid = CommunityIdentifier::from_str(cid).unwrap();
	if !cids.contains(&cid) {
		panic!("cid {cid} does not exist on chain");
	}
	cid
}

async fn get_block_number(api: &Api, maybe_at: Option<Hash>) -> BlockNumber {
	let hdr = api.get_header(maybe_at).await.unwrap().unwrap();
	debug!("decoded: {:?}", hdr);
	//let hdr: Header= Decode::decode(&mut .as_bytes()).unwrap();
	hdr.number
}

pub async fn get_community_balance(
	api: &Api,
	cid_str: &str,
	account_id: &AccountId,
	maybe_at: Option<Hash>,
) -> BalanceType {
	let cid = verify_cid(api, cid_str, maybe_at).await;
	let bn = get_block_number(api, maybe_at).await;
	let dr = get_demurrage_per_block(api, cid).await;

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
	let cid = verify_cid(api, cid_str, maybe_at).await;
	let bn = get_block_number(api, maybe_at).await;
	let dr = get_demurrage_per_block(api, cid).await;

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

async fn get_demurrage_per_block(api: &Api, cid: CommunityIdentifier) -> Demurrage {
	let d: Option<Demurrage> = api
		.get_storage_map("EncointerBalances", "DemurragePerBlock", cid, None)
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

async fn get_ceremony_index(api: &Api, at_block: Option<Hash>) -> CeremonyIndexType {
	api.get_storage("EncointerScheduler", "CurrentCeremonyIndex", at_block)
		.await
		.unwrap()
		.unwrap()
}

async fn get_attestee_count(api: &Api, key: CommunityCeremony) -> ParticipantIndexType {
	api.get_storage_map("EncointerCeremonies", "AttestationCount", key, None)
		.await
		.unwrap()
		.unwrap_or(0)
}

async fn get_attendees_for_community_ceremony(
	api: &Api,
	community_ceremony: CommunityCeremony,
	at_block: Option<Hash>,
) -> (Vec<AccountId>, Vec<AccountId>) {
	let key_prefix = api
		.get_storage_double_map_key_prefix(
			"EncointerCeremonies",
			"ParticipantReputation",
			community_ceremony,
		)
		.await
		.unwrap();
	let max_keys = 1000;
	let storage_keys = api
		.get_storage_keys_paged(Some(key_prefix), max_keys, None, at_block)
		.await
		.unwrap();

	if storage_keys.len() == max_keys as usize {
		error!("results can be wrong because max keys reached for query")
	}
	let mut attendees = Vec::new();
	let mut noshows = Vec::new();
	for storage_key in storage_keys.iter() {
		match api.get_storage_by_key(storage_key.clone(), at_block).await.unwrap().unwrap() {
			Reputation::VerifiedUnlinked | Reputation::VerifiedLinked(_) => {
				let key_postfix = storage_key.as_ref();
				attendees.push(
					AccountId::decode(&mut key_postfix[key_postfix.len() - 32..].as_ref()).unwrap(),
				);
			},
			Reputation::UnverifiedReputable | Reputation::Unverified => {
				let key_postfix = storage_key.as_ref();
				noshows.push(
					AccountId::decode(&mut key_postfix[key_postfix.len() - 32..].as_ref()).unwrap(),
				);
			},
		}
	}
	(attendees, noshows)
}

async fn get_reputation_lifetime(api: &Api, at_block: Option<Hash>) -> ReputationLifetimeType {
	api.get_storage("EncointerCeremonies", "ReputationLifetime", at_block)
		.await
		.unwrap()
		.unwrap_or(5)
}

async fn get_participant_attestation_index(
	api: &Api,
	key: CommunityCeremony,
	accountid: &AccountId,
) -> Option<ParticipantIndexType> {
	api.get_storage_double_map("EncointerCeremonies", "AttestationIndex", key, accountid, None)
		.await
		.unwrap()
}

async fn new_claim_for(
	api: &Api,
	claimant: &sr25519::Pair,
	cid: CommunityIdentifier,
	n_participants: u32,
) -> Vec<u8> {
	let cindex = get_ceremony_index(api, None).await;
	let mindex = api
		.get_meetup_index(&(cid, cindex), &claimant.public().into())
		.await
		.unwrap()
		.expect("participant must be assigned to meetup to generate a claim");

	// implicitly assume that participant meet at the right place at the right time
	let mloc = api.get_meetup_location(&(cid, cindex), mindex).await.unwrap().unwrap();
	let mtime = api.get_meetup_time(mloc, ONE_DAY).await.unwrap();

	info!(
		"creating claim for {} at loc {} (lat: {} lon: {}) at time {}, cindex {}",
		claimant.public().to_ss58check(),
		mindex,
		mloc.lat,
		mloc.lon,
		mtime,
		cindex
	);
	let claim: ClaimOfAttendance<MultiSignature, AccountId, Moment> =
		ClaimOfAttendance::new_unsigned(
			claimant.public().into(),
			cindex,
			cid,
			mindex,
			mloc,
			mtime,
			n_participants,
		)
		.sign(claimant);
	claim.encode()
}

async fn get_community_identifiers(
	api: &Api,
	maybe_at: Option<Hash>,
) -> Option<Vec<CommunityIdentifier>> {
	api.get_storage("EncointerCommunities", "CommunityIdentifiers", maybe_at)
		.await
		.unwrap()
}

/// This rpc needs to have offchain indexing enabled in the node.
async fn get_cid_names(api: &Api) -> Option<Vec<CidName>> {
	api.client().request("encointer_getAllCommunities", rpc_params![]).await.expect(
		"No communities returned. Are you running the node with `--enable-offchain-indexing true`?",
	)
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

async fn get_reputation_history(
	api: &Api,
	account_id: &AccountId,
) -> Option<Vec<(CeremonyIndexType, CommunityReputation)>> {
	api.client()
		.request("encointer_getReputations", rpc_params![account_id])
		.await
		.expect("Could not query reputation history...")
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

async fn get_asset_fee_details(
	api: &Api,
	cid_str: &str,
	encoded_xt: &Bytes,
) -> Option<FeeDetails<NumberOrHex>> {
	let cid = verify_cid(api, cid_str, None).await;

	api.client()
		.request("encointer_queryAssetFeeDetails", rpc_params![cid, encoded_xt])
		.await
		.expect("Could not query asset fee details")
}

fn prove_attendance(
	prover: AccountId,
	cid: CommunityIdentifier,
	cindex: CeremonyIndexType,
	attendee_str: &str,
) -> ProofOfAttendance<Signature, AccountId> {
	let msg = (prover.clone(), cindex);
	let attendee = get_pair_from_str(attendee_str);
	let attendeeid = get_accountid_from_str(attendee_str);
	debug!("generating proof of attendance for {} and cindex: {}", prover, cindex);
	debug!("signature payload is {:x?}", msg.encode());
	ProofOfAttendance {
		prover_public: prover,
		community_identifier: cid,
		ceremony_index: cindex,
		attendee_public: attendeeid,
		attendee_signature: Signature::from(sr25519_core::Signature::from(
			attendee.sign(&msg.encode()),
		)),
	}
}

async fn get_reputation(
	api: &Api,
	prover: &AccountId,
	cid: CommunityIdentifier,
	cindex: CeremonyIndexType,
) -> Reputation {
	api.get_storage_double_map(
		"EncointerCeremonies",
		"ParticipantReputation",
		(cid, cindex),
		prover.clone(),
		None,
	)
	.await
	.unwrap()
	.unwrap_or(Reputation::Unverified)
}

fn apply_demurrage(
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

async fn send_bazaar_xt(matches: &ArgMatches<'_>, bazaar_call: &BazaarCalls) -> Result<(), ()> {
	let business_owner = matches.account_arg().map(get_pair_from_str).unwrap();

	let mut api = get_chain_api(matches).await;
	api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(
		business_owner.clone(),
	)));
	let cid =
		verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"), None).await;
	let ipfs_cid = matches.ipfs_cid_arg().expect("ipfs cid needed");

	let tx_payment_cid_arg = matches.tx_payment_cid_arg();
	set_api_extrisic_params_builder(&mut api, tx_payment_cid_arg);
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

async fn endorse_newcomers(
	api: &mut Api,
	cid: CommunityIdentifier,
	matches: &ArgMatches<'_>,
) -> Result<(), ApiClientError> {
	let bootstrapper = matches.bootstrapper_arg().map(get_pair_from_str).unwrap();
	let endorsees = matches.endorsees_arg().expect("Please supply at least one endorsee");

	api.set_signer(ParentchainExtrinsicSigner::new(sr25519_core::Pair::from(bootstrapper)));

	let mut nonce = api.get_nonce().await?;

	let tx_payment_cid_arg = matches.tx_payment_cid_arg();
	set_api_extrisic_params_builder(api, tx_payment_cid_arg);

	for e in endorsees.into_iter() {
		let endorsee = get_accountid_from_str(e);

		let call =
			compose_call!(api.metadata(), "EncointerCeremonies", "endorse_newcomer", cid, endorsee)
				.unwrap();

		let encoded_xt: Bytes = api.compose_extrinsic_offline(call, nonce).encode().into();
		ensure_payment(api, &encoded_xt, tx_payment_cid_arg).await;
		let _tx_report = api
			.submit_and_watch_opaque_extrinsic_until(&encoded_xt, XtStatus::Ready)
			.await
			.unwrap();

		nonce += 1;
	}

	Ok(())
}

/// Helper type, which is only needed to print the information nicely.
#[derive(Debug)]
struct BootstrapperWithTickets {
	bootstrapper: AccountId,
	remaining_newbie_tickets: u8,
}

async fn get_bootstrappers_with_remaining_newbie_tickets(
	api: &Api,
	cid: CommunityIdentifier,
) -> Result<Vec<BootstrapperWithTickets>, ApiClientError> {
	let total_newbie_tickets: u8 = api
		.get_storage("EncointerCeremonies", "EndorsementTicketsPerBootstrapper", None)
		.await
		.unwrap()
		.unwrap();

	// prepare closure to make below call more readable.
	let ticket_query = |bs| async move {
		let remaining_tickets = total_newbie_tickets
			- api
				.get_storage_double_map(
					"EncointerCeremonies",
					"BurnedBootstrapperNewbieTickets",
					cid,
					bs,
					None,
				)
				.await?
				.unwrap_or(0u8);

		Ok::<_, ApiClientError>(remaining_tickets)
	};

	let bootstrappers: Vec<AccountId> = api
		.get_storage_map("EncointerCommunities", "Bootstrappers", cid, None)
		.await?
		.expect("No bootstrappers found, does the community exist?");

	let mut bs_with_tickets: Vec<BootstrapperWithTickets> = Vec::with_capacity(bootstrappers.len());

	for bs in bootstrappers.into_iter() {
		bs_with_tickets.push(BootstrapperWithTickets {
			bootstrapper: bs.clone(),
			remaining_newbie_tickets: ticket_query(bs).await?,
		});
	}

	Ok(bs_with_tickets)
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

async fn set_api_extrisic_params_builder(api: &mut Api, tx_payment_cid_arg: Option<&str>) {
	let mut tx_params = CommunityCurrencyTipExtrinsicParamsBuilder::new().tip(0);
	if let Some(tx_payment_cid) = tx_payment_cid_arg {
		tx_params = tx_params.tip(
			CommunityCurrencyTip::new(0).of_community(verify_cid(api, tx_payment_cid, None).await),
		);
	}
	let _ = &api.set_additional_params(tx_params);
}
