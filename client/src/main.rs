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
		add_location_call, new_community_call, read_community_spec_from_file, CommunitySpec,
	},
	utils::{
		batch_call, collective_propose_call, contains_sudo_pallet, ensure_payment, get_councillors,
		into_effective_cindex,
		keys::{get_accountid_from_str, get_pair_from_str},
		offline_xt, print_raw_call, send_and_wait_for_in_block, sudo_call, xt, OpaqueCall,
	},
};
use clap::{value_t, AppSettings, Arg, ArgMatches};
use clap_nested::{Command, Commander};
use cli_args::{EncointerArgs, EncointerArgsExtractor};
use codec::{Compact, Decode, Encode};
use encointer_api_client_extension::{
	CeremoniesApi, CommunitiesApi, SchedulerApi, ENCOINTER_CEREMONIES,
};
use encointer_node_notee_runtime::{
	AccountId, BalanceEntry, BalanceType, BlockNumber, Event, Hash, Header, Moment, Signature,
	ONE_DAY,
};
use encointer_primitives::{
	balances::Demurrage,
	bazaar::{BusinessData, BusinessIdentifier, OfferingData},
	ceremonies::{
		AttestationIndexType, ClaimOfAttendance, CommunityCeremony, CommunityReputation,
		ParticipantIndexType, ProofOfAttendance, Reputation,
	},
	communities::{CidName, CommunityIdentifier},
	fixed::transcendental::exp,
	scheduler::{CeremonyIndexType, CeremonyPhaseType},
};
use log::*;
use serde_json::{json, to_value};
use sp_application_crypto::{ed25519, sr25519};
use sp_core::{crypto::Ss58Codec, sr25519 as sr25519_core, Pair};
use sp_keyring::AccountKeyring;
use sp_runtime::{traits::Verify, MultiSignature};
use std::{
	collections::HashMap, convert::TryInto, path::PathBuf, str::FromStr, sync::mpsc::channel,
};
use substrate_api_client::{
	compose_call, compose_extrinsic, compose_extrinsic_offline, rpc::WsRpcClient,
	utils::FromHexString, Api, ApiClientError, ApiResult, GenericAddress, Metadata,
	UncheckedExtrinsicV4, XtStatus,
};
use substrate_client_keystore::{KeystoreExt, LocalKeystore};

type AccountPublic = <Signature as Verify>::Signer;
const KEYSTORE_PATH: &str = "my_keystore";
const PREFUNDING_NR_OF_TRANSFER_EXTRINSICS: u128 = 1000;
const VERSION: &str = env!("CARGO_PKG_VERSION");

mod exit_code {
	pub const WRONG_PHASE: i32 = 50;
	pub const FEE_PAYMENT_FAILED: i32 = 51;
	pub const INVALID_REPUTATION: i32 = 52;
	pub const RPC_ERROR: i32 = 60;
}

fn main() {
	env_logger::init();

	let _ = Commander::new()
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
            .name("encointer-client-notee")
            .version(VERSION)
            .author("Encointer Association <info@encointer.org>")
            .about("interact with encointer-node-notee")
            .after_help("")
            .setting(AppSettings::ColoredHelp)
        })
        .args(|_args, _matches| "")
        .add_cmd(
            Command::new("new-account")
                .description("generates a new account")
                .runner(|_args: &str, _matches: &ArgMatches<'_>| {
                    let store = LocalKeystore::open(PathBuf::from(&KEYSTORE_PATH), None).unwrap();
                    let key: sr25519::AppPair = store.generate().unwrap();
                    drop(store);
                    println!("{}", key.public().to_ss58check());
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("list-accounts")
                .description("lists all accounts in keystore")
                .runner(|_args: &str, _matches: &ArgMatches<'_>| {
                    let store = LocalKeystore::open(PathBuf::from(&KEYSTORE_PATH), None).unwrap();
                    info!("sr25519 keys:");
                    for pubkey in store.public_keys::<sr25519::AppPublic>()
                        .unwrap()
                        .into_iter()
                    {
                        println!("{}", pubkey.to_ss58check());
                    }
                    info!("ed25519 keys:");
                    for pubkey in store.public_keys::<ed25519::AppPublic>()
                        .unwrap()
                        .into_iter()
                    {
                        println!("{}", pubkey.to_ss58check());
                    }
                    drop(store);
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("print-metadata")
                .description("query node metadata and print it as json to stdout")
                .runner(|_args: &str, matches: &ArgMatches<'_>| {
                    let meta = get_chain_api(matches).get_metadata().unwrap();
                    println!("Metadata:\n {}", Metadata::pretty_format(&meta).unwrap());
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("faucet")
                .description("send some bootstrapping funds to supplied account(s)")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                    .fundees_arg()
                })
                .runner(|_args: &str, matches: &ArgMatches<'_>| {
                    let api = get_chain_api(matches)
                        .set_signer(AccountKeyring::Alice.pair());
                    let accounts = matches.fundees_arg().unwrap();

                    let existential_deposit = api.get_existential_deposit().unwrap();
                    info!("Existential deposit is = {:?}", existential_deposit);

                    let mut nonce = api.get_nonce().unwrap();

                    let amount = reasonable_native_balance(&api);
                    for account in accounts.into_iter() {
                        let to = get_accountid_from_str(account);
                        let call = compose_call!(
                            api.metadata,
                            "Balances",
                            "transfer",
                            GenericAddress::Id(to.clone()),
                            Compact(amount)
                        );
                        let xt: UncheckedExtrinsicV4<_> = compose_extrinsic_offline!(
                            api.clone().signer.unwrap(),
                            call.clone(),
                            nonce,
                            Era::Immortal,
                            api.genesis_hash,
                            api.genesis_hash,
                            api.runtime_version.spec_version,
                            api.runtime_version.transaction_version
                        );
                        ensure_payment(&api, &xt.hex_encode());
                        // send and watch extrinsic until finalized
                        println!("Faucet drips {} to {} (Alice's nonce={})", amount, to, nonce);
                        let _blockh = api
                            .send_extrinsic(xt.hex_encode(), XtStatus::Ready)
                            .unwrap();
                        nonce += 1;
                    }
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("balance")
                .description("query on-chain balance for AccountId. If --cid is supplied, returns balance in that community. Otherwise balance of native ERT token")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                    .account_arg()
                    .all_flag()
                })
                .runner(|_args: &str, matches: &ArgMatches<'_>| {
                    let api = get_chain_api(matches);
                    let account = matches.account_arg().unwrap();
                    let accountid = get_accountid_from_str(account);
                    match matches.cid_arg() {
                        Some(cid_str) => {
                            let cid = verify_cid(&api, cid_str);
                            let bn = get_block_number(&api);
                            let dr = get_demurrage_per_block(&api, cid);
                            let balance = if let Some(entry) = api
                                .get_storage_double_map("EncointerBalances", "Balance", cid, accountid, None).unwrap() {
                                    apply_demurrage(entry, bn, dr)
                            } else { BalanceType::from_num(0) };
                            println!("{}", balance);
                        }
                        None => {
                            if matches.all_flag() {
                                let community_balances = get_all_balances(&api, &accountid).unwrap();
                                let bn = get_block_number(&api);
                                for b in community_balances.iter() {
                                    let dr = get_demurrage_per_block(&api, b.0);
                                    println!("{}: {}", b.0, apply_demurrage(b.1, bn, dr))
                                }
                            }
                            let balance = if let Some(data) = api.get_account_data(&accountid).unwrap() {
                                data.free
                            } else {
                                0
                            };
                            println!("{}", balance);
                        }
                    };
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("transfer")
                .description("transfer funds from one account to another. If --cid is supplied, send that community (amount is fixpoint). Otherwise send native ERT tokens (amount is integer)")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                    .arg(
                        Arg::with_name("from")
                            .takes_value(true)
                            .required(true)
                            .value_name("SS58")
                            .help("sender's AccountId in ss58check format"),
                    )
                    .arg(
                        Arg::with_name("to")
                            .takes_value(true)
                            .required(true)
                            .value_name("SS58")
                            .help("recipient's AccountId in ss58check format"),
                    )
                    .arg(
                        Arg::with_name("amount")
                            .takes_value(true)
                            .required(true)
                            .value_name("U128")
                            .help("amount to be transferred"),
                    )
                })
                .runner(|_args: &str, matches: &ArgMatches<'_>| {
                    let api = get_chain_api(matches);
                    let arg_from = matches.value_of("from").unwrap();
                    let arg_to = matches.value_of("to").unwrap();
                    let from = get_pair_from_str(arg_from);
                    let to = get_accountid_from_str(arg_to);
                    info!("from ss58 is {}", from.public().to_ss58check());
                    info!("to ss58 is {}", to.to_ss58check());
                    let _api = api.set_signer(sr25519_core::Pair::from(from));
                    let tx_hash = match matches.cid_arg() {
                        Some(cid_str) => {
                            let cid = verify_cid(&_api, cid_str);
                            let amount = BalanceType::from_str(matches.value_of("amount").unwrap())
                                .expect("amount can be converted to fixpoint");
                            let xt: UncheckedExtrinsicV4<_> = compose_extrinsic!(
                                _api.clone(),
                                "EncointerBalances",
                                "transfer",
                                to.clone(),
                                cid,
                                amount
                            );
                            // ensure_payment(&_api, &xt.hex_encode());
                            _api.send_extrinsic(xt.hex_encode(), XtStatus::InBlock).unwrap()
                        },
                        None => {
                            let amount = u128::from_str_radix(matches.value_of("amount").unwrap(), 10)
                                .expect("amount can be converted to u128");
                            let xt = _api.balance_transfer(
                                GenericAddress::Id(to.clone()),
                                amount
                            );
                            ensure_payment(&_api, &xt.hex_encode());
                            _api.send_extrinsic(xt.hex_encode(), XtStatus::InBlock).unwrap()
                        }
                    };
                    info!("[+] Transaction included. Hash: {:?}\n", tx_hash);
                    let result = _api.get_account_data(&to.clone()).unwrap().unwrap();
                    println!("balance for {} is now {}", to, result.free);
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("listen")
                .description("listen to on-chain events")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                    .arg(
                        Arg::with_name("events")
                            .short("e")
                            .long("await-events")
                            .takes_value(true)
                            .help("exit after given number of encointer events"),
                    )
                    .arg(
                        Arg::with_name("blocks")
                            .short("b")
                            .long("await-blocks")
                            .takes_value(true)
                            .help("exit after given number of blocks"),
                    )
                })
                .runner(|_args: &str, matches: &ArgMatches<'_>| {
                    listen(matches);
                    Ok(())
                }),
        )
        // start encointer stuff
        .add_cmd(
            Command::new("new-community")
                .description("Register new community")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                    .arg(
                        Arg::with_name("specfile")
                            .takes_value(true)
                            .required(true)
                            .help("enhanced geojson file that specifies a community"),
                    )
                })
                .runner(|_args: &str, matches: &ArgMatches<'_>| {
                    // -----setup
                    let spec_file = matches.value_of("specfile").unwrap();
                    let spec = read_community_spec_from_file(spec_file);
                    let cid = spec.community_identifier();

                    let sudoer = AccountKeyring::Alice.pair();
                    let api = get_chain_api(matches).set_signer(sudoer);

                    // ------- create calls for xt's

                    let new_community_call = new_community_call(&spec, &api.metadata);
                    // only the first meetup location has been registered now. register all others one-by-one
                    let add_location_calls = spec.locations().into_iter().skip(1).map(|l| add_location_call(&api.metadata, cid, l)).collect();
                    let add_location_batch_call = batch_call(&api.metadata, add_location_calls);

                    // return calls as `OpaqueCall`s to get the same return type in both branches
                    let (new_community_call, add_location_batch_call) = if contains_sudo_pallet(&api.metadata) {
                        let sudo_new_community = sudo_call(&api.metadata, new_community_call);
                        let sudo_add_location_batch = sudo_call(&api.metadata, add_location_batch_call);
                        info!("Printing raw sudo calls for js/apps for cid: {}", cid);
                        print_raw_call("sudo(new_community)", &sudo_new_community);
                        print_raw_call("sudo(utility_batch(add_location))", &sudo_add_location_batch);

                        (OpaqueCall::from_tuple(&sudo_new_community), OpaqueCall::from_tuple(&sudo_add_location_batch))

                    } else {
                        let threshold = (get_councillors(&api).unwrap().len() / 2 + 1) as u32;
                        info!("Printing raw collective propose calls with threshold {} for js/apps for cid: {}", threshold, cid);
                        let propose_new_community = collective_propose_call(&api.metadata, threshold, new_community_call);
                        let propose_add_location_batch = collective_propose_call(&api.metadata, threshold, add_location_batch_call);
                        print_raw_call("collective_propose(new_community)", &propose_new_community);
                        print_raw_call("collective_propose(utility_batch(add_location))", &propose_add_location_batch);

                        (OpaqueCall::from_tuple(&propose_new_community), OpaqueCall::from_tuple(&propose_add_location_batch))
                    };

                    // ---- send xt's to chain
                    send_and_wait_for_in_block(&api, xt(&api, new_community_call));
                    println!("{}", cid);

                    if api.get_current_phase().unwrap() != CeremonyPhaseType::Registering {
                        error!("Wrong ceremony phase for registering new locations for {}", cid);
                        error!("Aborting without registering additional locations");
                        std::process::exit(exit_code::WRONG_PHASE);
                    }
                    send_and_wait_for_in_block(&api, xt(&api, add_location_batch_call));
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("add-locations")
                .description("Register new locations for a community")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                        .arg(
                            Arg::with_name("specfile")
                                .takes_value(true)
                                .required(true)
                                .help("geojson file that specifies locations to add as points"),
                        )
                })
                .runner(|_args: &str, matches: &ArgMatches<'_>| {
                    // -----setup
                    let spec_file = matches.value_of("specfile").unwrap();
                    let spec = read_community_spec_from_file(spec_file);

                    let sudoer = AccountKeyring::Alice.pair();
                    let api = get_chain_api(matches).set_signer(sudoer);

                    let cid = verify_cid(&api, matches.cid_arg().unwrap());

                    let add_location_calls = spec.locations().into_iter().skip(1).map(|l| add_location_call(&api.metadata, cid, l)).collect();
                    let add_location_batch_call = batch_call(&api.metadata, add_location_calls);

                    // return calls as `OpaqueCall`s to get the same return type in both branches
                    let add_location_batch_call = if contains_sudo_pallet(&api.metadata) {
                        let sudo_add_location_batch = sudo_call(&api.metadata, add_location_batch_call);
                        info!("Printing raw sudo calls for js/apps for cid: {}", cid);
                        print_raw_call("sudo(utility_batch(add_location))", &sudo_add_location_batch);
                        OpaqueCall::from_tuple(&sudo_add_location_batch)
                    } else {
                        let threshold = (get_councillors(&api).unwrap().len() / 2 + 1) as u32;
                        info!("Printing raw collective propose calls with threshold {} for js/apps for cid: {}", threshold, cid);
                        let propose_add_location_batch = collective_propose_call(&api.metadata, threshold, add_location_batch_call);
                        print_raw_call("collective_propose(utility_batch(add_location))", &propose_add_location_batch);
                        OpaqueCall::from_tuple(&propose_add_location_batch)
                    };
                    // ---- send xt's to chain
                    if api.get_current_phase().unwrap() != CeremonyPhaseType::Registering {
                        error!("Wrong ceremony phase for registering new locations for {}", cid);
                        error!("Aborting without registering additional locations");
                        std::process::exit(exit_code::WRONG_PHASE);
                    }
                    send_and_wait_for_in_block(&api, xt(&api, add_location_batch_call));
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("list-communities")
                .description("list all registered communities")
                .runner(|_args: &str, matches: &ArgMatches<'_>| {
                    let api = get_chain_api(matches);
                    let names = get_cid_names(&api).unwrap();
                    println!("number of communities:  {}", names.len());
                    for n in names.iter() {
                        let loc = api.get_locations(n.cid).unwrap();
                        println!("{}: {} locations: {}", n.cid, n.name, loc.len());
                    }
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("list-locations")
                .description("list all meetup locations for a community")
                .runner(|_args: &str, matches: &ArgMatches<'_>| {
                    let api = get_chain_api(matches);
                    let cid = verify_cid(&api,
                                         matches
                                             .cid_arg()
                                             .expect("please supply argument --cid"),
                    );
                    println!("listing locations for cid {}", cid);
                    let loc = api.get_locations(cid).unwrap();
                    for l in loc.iter() {
                        println!("lat: {} lon: {}", l.lat, l.lon);
                    }
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("get-phase")
                .description("read current ceremony phase from chain")
                .runner(|_args: &str, matches: &ArgMatches<'_>| {
                    let api = get_chain_api(matches);

                    // >>>> add some debug info as well
                    let bn = get_block_number(&api);
                    debug!("block number: {}", bn);
                    let cindex = get_ceremony_index(&api);
                    info!("ceremony index: {}", cindex);
                    let tnext: Moment = api.get_next_phase_timestamp().unwrap();
                    debug!("next phase timestamp: {}", tnext);
                    // <<<<

                    let phase = api.get_current_phase().unwrap();
                    println!("{:?}", phase);
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("next-phase")
                .description("Advance ceremony state machine to next phase by ROOT call")
                .runner(|_args: &str, matches: &ArgMatches<'_>| {
                    let api = get_chain_api(matches)
                        .set_signer(AccountKeyring::Alice.pair());
                    let next_phase_call = compose_call!(
                        &api.metadata,
                        "EncointerScheduler",
                        "next_phase"
                    );

                    // return calls as `OpaqueCall`s to get the same return type in both branches
                    let next_phase_call = if contains_sudo_pallet(&api.metadata) {
                        let sudo_next_phase_call = sudo_call(&api.metadata, next_phase_call);
                        info!("Printing raw sudo call for js/apps:");
                        print_raw_call("sudo(next_phase)", &sudo_next_phase_call);

                        OpaqueCall::from_tuple(&sudo_next_phase_call)

                    } else {
                        let threshold = (get_councillors(&api).unwrap().len() / 2 + 1) as u32;
                        info!("Printing raw collective propose calls with threshold {} for js/apps", threshold);
                        let propose_next_phase = collective_propose_call(&api.metadata, threshold, next_phase_call);
                        print_raw_call("collective_propose(next_phase)", &propose_next_phase);

                        OpaqueCall::from_tuple(&propose_next_phase)
                    };

                    send_and_wait_for_in_block(&api, xt(&api, next_phase_call));

                    let phase = api.get_current_phase().unwrap();
                    println!("Phase is now: {:?}", phase);
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("list-participants")
                .description("list all registered participants supplied community identifier and ceremony index")
                .options(|app| {
                app.setting(AppSettings::ColoredHelp)
                    .ceremony_index_arg()
                })
                .runner(|_args: &str, matches: &ArgMatches<'_>| {
                    extract_and_execute(
                        &matches, |api, cid| -> ApiResult<()>{

                            let current_ceremony_index = get_ceremony_index(&api);

                            let cindex = matches.ceremony_index_arg()
                                .map_or_else(|| current_ceremony_index , |ci| into_effective_cindex(ci, current_ceremony_index));

                            println!("listing participants for cid {} and ceremony nr {}", cid, cindex);

                            let counts = vec!["BootstrapperCount", "ReputableCount", "EndorseeCount", "NewbieCount"];
                            let count_query = |count_index| api.get_storage_map(ENCOINTER_CEREMONIES, counts[count_index], (cid, cindex), None);

                            let registries = vec!["BootstrapperRegistry", "ReputableRegistry", "EndorseeRegistry", "NewbieRegistry"];
                            let account_query = |registry_index, p_index| api.get_storage_double_map(ENCOINTER_CEREMONIES, registries[registry_index],(cid, cindex), p_index, None);

                            for i in 0..registries.len() {
                                println!("Querying {}", registries[i]);

                                let count: ParticipantIndexType = count_query(i)?.unwrap_or(0);
                                println!("number of participants assigned:  {}", count);

                                for p_index in 1..count +1 {
                                    let accountid: AccountId = account_query(i, p_index)?.unwrap();
                                    println!("{}[{}, {}] = {}", registries[i], cindex, p_index, accountid);
                                }
                            }

                            Ok(())
                        }
                    ).unwrap();

                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("list-meetups")
                .description("list all assigned meetups for supplied community identifier and ceremony index")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                        .ceremony_index_arg()
                })
                .runner(|_args: &str, matches: &ArgMatches<'_>| {
                    extract_and_execute(
                        &matches, |api, cid| -> ApiResult<()>{

                            let current_ceremony_index = get_ceremony_index(&api);

                            let cindex = matches.ceremony_index_arg()
                                .map_or_else(|| current_ceremony_index , |ci| into_effective_cindex(ci, current_ceremony_index));

                            let community_ceremony = (cid, cindex);

                            println!("listing meetups for cid {} and ceremony nr {}", cid, cindex);

                            let stats = api.get_community_ceremony_stats(community_ceremony.clone()).unwrap();

                            for meetup in stats.meetups.iter() {
                                println!("MeetupRegistry[{:?}, {}] location is {:?}", &community_ceremony, meetup.index, meetup.location);

                                println!("MeetupRegistry[{:?}, {}] meeting time is {:?}", &community_ceremony, meetup.index, meetup.time);

                                if !meetup.registrations.is_empty() {
                                    println!("MeetupRegistry[{:?}, {}] participants are:", &community_ceremony, meetup.index);
                                    for (participant, _registration) in meetup.registrations.iter() {
                                        println!("   {}", participant);
                                    }
                                } else {
                                    println!("MeetupRegistry[{:?}, {}] EMPTY", &community_ceremony, meetup.index);
                                }
                            }

                            Ok(())
                        }
                    ).unwrap();

                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("print-ceremony-stats")
                .description("pretty prints all information for a community ceremony")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                        .ceremony_index_arg()
                })
                .runner(|_args: &str, matches: &ArgMatches<'_>| {
                    extract_and_execute(
                        &matches, |api, cid| -> ApiResult<()>{

                            let current_ceremony_index = get_ceremony_index(&api);

                            let cindex = matches.ceremony_index_arg()
                                .map_or_else(|| current_ceremony_index , |ci| into_effective_cindex(ci, current_ceremony_index));

                            let community_ceremony = (cid, cindex);

                            let stats = api.get_community_ceremony_stats(community_ceremony.clone()).unwrap();

                            // serialization prints the the account id better than `debug`
                            println!("{}", serde_json::to_string_pretty(&stats).unwrap());
                            Ok(())
                        }
                    ).unwrap();

                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("list-attestees")
                .description("list all attestees for participants for supplied community identifier and ceremony index")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                        .ceremony_index_arg()
                })
                .runner(|_args: &str, matches: &ArgMatches<'_>| {
                    extract_and_execute(
                        &matches, |api, cid| -> ApiResult<()>{

                            let current_ceremony_index = get_ceremony_index(&api);

                            let cindex = matches.ceremony_index_arg()
                                .map_or_else(|| current_ceremony_index , |ci| into_effective_cindex(ci, current_ceremony_index));

                            println!("listing attestees for cid {} and ceremony nr {}", cid, cindex);

                            let wcount = get_attestee_count(&api, (cid, cindex));
                            println!("number of attestees:  {}", wcount);

                            println!("listing participants for cid {} and ceremony nr {}", cid, cindex);

                            let counts = vec!["BootstrapperCount", "ReputableCount", "EndorseeCount", "NewbieCount"];
                            let count_query = |count_index| api.get_storage_map(ENCOINTER_CEREMONIES, counts[count_index], (cid, cindex), None);

                            let registries = vec!["BootstrapperRegistry", "ReputableRegistry", "EndorseeRegistry", "NewbieRegistry"];
                            let account_query = |registry_index, p_index| api.get_storage_double_map(ENCOINTER_CEREMONIES, registries[registry_index],(cid, cindex), p_index, None);

                            let mut participants_windex = HashMap::new();

                            for i in 0..registries.len() {
                                println!("Querying {}", registries[i]);

                                let count: ParticipantIndexType = count_query(i)?.unwrap_or(0);
                                println!("number of participants assigned:  {}", count);

                                for p_index in 1..count +1 {
                                    let accountid: AccountId = account_query(i, p_index)?.unwrap();

                                    match get_participant_attestation_index(&api, (cid, cindex), &accountid) {
                                        Some(windex) => {
                                            participants_windex.insert(windex as AttestationIndexType, accountid)
                                        }
                                        _ => continue,
                                    };
                                }
                            }

                            for w in 1..wcount + 1 {
                                let attestees = get_attestees(&api, (cid, cindex), w);
                                println!(
                                    "AttestationRegistry[{}, {} ({})] = {:?}",
                                    cindex, w, participants_windex[&w], attestees
                                );
                            }
                            Ok(())
                        }
                    ).unwrap();

                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("register-participant")
                .description("Register encointer ceremony participant for supplied community")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                    .account_arg()
                })
                .runner(move |_args: &str, matches: &ArgMatches<'_>| {
                    let arg_who = matches.account_arg().unwrap();
                    let accountid = get_accountid_from_str(arg_who);
                    let signer = get_pair_from_str(arg_who);
                    let api = get_chain_api(matches);
                    let cindex = get_ceremony_index(&api);
                    let cid = verify_cid(&api,
                        matches
                            .cid_arg()
                            .expect("please supply argument --cid"),
                    );
                    let rep = get_reputation(&api, &accountid, cid, cindex -1);
                    info!("{} has reputation {:?}", accountid, rep);
                    let proof = match rep {
                        Reputation::Unverified => None,
                        Reputation::UnverifiedReputable => None, // this should never by the case during Registering!
                        Reputation::VerifiedUnlinked => Some(prove_attendance(accountid, cid, cindex - 1, arg_who)),
                        Reputation::VerifiedLinked => {
                            error!("reputation of {} has already been linked! Not registering again", accountid);
                            std::process::exit(exit_code::INVALID_REPUTATION);
                        },
                    };
                    debug!("proof: {:x?}", proof.encode());
                    if api.get_current_phase().unwrap() != CeremonyPhaseType::Registering {
                        error!("wrong ceremony phase for registering participant");
                        std::process::exit(exit_code::WRONG_PHASE);
                    }
                    let _api = api.clone().set_signer(sr25519_core::Pair::from(signer.clone()));
                    let xt: UncheckedExtrinsicV4<_> = compose_extrinsic!(
                        _api.clone(),
                        "EncointerCeremonies",
                        "register_participant",
                        cid,
                        proof
                    );
                    ensure_payment(&_api, &xt.hex_encode());
                    // send and watch extrinsic until finalized
                    let _ = _api.send_extrinsic(xt.hex_encode(), XtStatus::Ready).unwrap();
                    info!("Registration sent for {}. status: 'ready'", arg_who);
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("endorse-newcomers")
                .description("Endorse newbies with a bootstrapper account")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                        .bootstrapper_arg()
                        .endorsees_arg()
                })
                .runner(move |_args: &str, matches: &ArgMatches<'_>| {

                    extract_and_execute(
                        &matches, |mut api, cid| endorse_newcomers(&mut api, cid, &matches)
                    ).unwrap();

                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("get-bootstrappers-with-remaining-newbie-tickets")
                .description("Get the bootstrappers along with the remaining newbie tickets")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                })
                .runner(|_args: &str, matches: &ArgMatches<'_>| {
                    let bs_with_tickets : Vec<BootstrapperWithTickets> = extract_and_execute(
                        &matches, |mut api, cid| get_bootstrappers_with_remaining_newbie_tickets(&mut api, cid)
                    ).unwrap();

                    info!("burned_bootstrapper_newbie_tickets = {:?}", bs_with_tickets);

                    // transform it to simple tuples, which is easier to parse in python
                    let bt_vec = bs_with_tickets.into_iter()
                        .map(|bt| (bt.bootstrapper.to_ss58check(), bt.remaining_newbie_tickets)).collect::<Vec<_>>();

                    println!("{:?}", bt_vec);
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("get-proof-of-attendance")
                .description("creates a proof of ProofOfAttendances for an <account> for the given ceremony index")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                        .setting(AppSettings::AllowLeadingHyphen)
                        .account_arg()
                        .ceremony_index_arg()
                })
                .runner(move |_args: &str, matches: &ArgMatches<'_>| {
                    let arg_who = matches.account_arg().unwrap();
                    let accountid = get_accountid_from_str(arg_who);
                    let api = get_chain_api(matches);

                    let current_ceremony_index = get_ceremony_index(&api);

                    let cindex_arg = matches.ceremony_index_arg().unwrap_or(-1);
                    let cindex = into_effective_cindex(cindex_arg, current_ceremony_index);

                    let cid = verify_cid(
                        &api,
                     matches.cid_arg().expect("please supply argument --cid"),
                    );

                    debug!("Getting proof for ceremony index: {:?}", cindex);
                    let proof = prove_attendance(accountid.clone(), cid, cindex, arg_who);
                    info!("Proof: {:?}\n", &proof);
                    println!("0x{}", hex::encode(proof.encode()));

                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("attest-claims")
                .description("Register encointer ceremony claim of attendances for supplied community")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                    .account_arg()
                    .claims_arg()
                })
                .runner(move |_args: &str, matches: &ArgMatches<'_>| {
                    let who = matches.account_arg().map(get_pair_from_str).unwrap();

                    let claims: Vec<ClaimOfAttendance<MultiSignature, AccountId, Moment>> = matches.claims_arg().unwrap()
                        .into_iter()
                        .map(|c| Decode::decode(&mut &hex::decode(c).unwrap()[..]).unwrap())
                        .collect();

                    debug!("claims: {:?}", claims);

                    info!("send attest_claims by {}", who.public());

                    let api = get_chain_api(matches).set_signer(who.clone().into());
                    let xt: UncheckedExtrinsicV4<_> = compose_extrinsic!(
                        api.clone(),
                        "EncointerCeremonies",
                        "attest_claims",
                        claims.clone()
                    );

                    ensure_payment(&api, &xt.hex_encode());
                    let _ = api.send_extrinsic(xt.hex_encode(), XtStatus::Ready).unwrap();

                    println!("Claims sent by {}. status: 'ready'", who.public());
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("new-claim")
                .description("create a fresh claim of attendance for account")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                    .account_arg()
                    .arg(
                        Arg::with_name("vote")
                            .takes_value(true)
                            .required(true)
                            .value_name("VOTE")
                            .help("participant's vote on the number of people present at meetup time"),
                    )
                })
                .runner(move |_args: &str, matches: &ArgMatches<'_>| {
                    extract_and_execute(
                        &matches, |api, cid| -> ApiResult<()>{
                            let arg_who = matches.account_arg().unwrap();
                            let claimant = get_pair_from_str(arg_who);

                            let n_participants = matches
                                .value_of("vote")
                                .unwrap()
                                .parse::<u32>()
                                .unwrap();

                            let claim = new_claim_for(&api, &claimant.into(), cid, n_participants);

                            println!("{}", hex::encode(claim));
                            Ok(())
                        }
                    ).unwrap();

                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("claim-reward")
                .description("Claim the rewards for all meetup participants of the last ceremony.")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                        .signer_arg("account that was part of the meetup")
                })
                .runner(move |_args: &str, matches: &ArgMatches<'_>| {

                    extract_and_execute(
                        &matches, |api, cid| {
                            let signer = matches.signer_arg().map(get_pair_from_str).unwrap();
                            let api = api.set_signer(signer.clone().into());

                            let xt: UncheckedExtrinsicV4<_> = compose_extrinsic!(
                                api.clone(),
                                ENCOINTER_CEREMONIES,
                                "claim_rewards",
                                cid
                            );
                            ensure_payment(&api, &xt.hex_encode());

                            let _ = api.send_extrinsic(xt.hex_encode(), XtStatus::Ready).unwrap();
                            println!("Claiming reward for {}. xt-status: 'ready'", signer.public());
                        }
                    );

                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("reputation")
                .description("List reputation history for an account")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                        .account_arg()})
                .runner(move |_args: &str, matches: &ArgMatches<'_>| {
                    let api = get_chain_api(matches);
                    let account = matches.account_arg().unwrap();
                    let account_id = get_accountid_from_str(account);
                    if let Some(reputation) = get_reputation_history(&api, &account_id) {
                        for rep in reputation.iter() {
                            println!("{}, {}, {:?}", rep.0, rep.1.community_identifier, rep.1.reputation);
                        }
                    } else {
                        error!("could not fetch reputation over rpc");
                        std::process::exit(exit_code::RPC_ERROR);
                    }
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("create-business")
                .description("Register a community business on behalf of the account")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                        .account_arg()
                        .ipfs_cid_arg()
                })
                .runner(move |_args: &str, matches: &ArgMatches<'_>| {
                    send_bazaar_xt(&matches, &BazaarCalls::CreateBusiness).unwrap();
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("update-business")
                .description("Update an already existing community business on behalf of the account")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                        .account_arg()
                        .ipfs_cid_arg()
                })
                .runner(move |_args: &str, matches: &ArgMatches<'_>| {
                    send_bazaar_xt(&matches, &BazaarCalls::UpdateBusiness).unwrap();
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("create-offering")
                .description("Create an offering for the business belonging to account")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                        .account_arg()
                        .ipfs_cid_arg()
                })
                .runner(move |_args: &str, matches: &ArgMatches<'_>| {
                    send_bazaar_xt(&matches, &BazaarCalls::CreateOffering).unwrap();
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("list-businesses")
                .description("List businesses for a community")
                .runner(move |_args: &str, matches: &ArgMatches<'_>| {

                    let businesses = extract_and_execute(
                        &matches, |api, cid| get_businesses(&api, cid).unwrap()
                    );
                    // only print plain businesses to be able to parse them in python scripts
                    println!("{:?}", businesses);
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("list-offerings")
                .description("List offerings for a community")
                .runner(move |_args: &str, matches: &ArgMatches<'_>| {
                    let offerings = extract_and_execute(
                        &matches, |api, cid| get_offerings(&api, cid).unwrap()
                    );
                    // only print plain offerings to be able to parse them in python scripts
                    println!("{:?}", offerings);
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("list-business-offerings")
                .description("List offerings for a business")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                        .account_arg()
                })
                .runner(move |_args: &str, matches: &ArgMatches<'_>| {
                    let account = matches.account_arg().map(get_accountid_from_str).unwrap();

                    let offerings = extract_and_execute(
                        &matches, |api, cid| get_offerings_for_business(&api, cid, account).unwrap()
                    );
                    // only print plain offerings to be able to parse them in python scripts
                    println!("{:?}", offerings);
                    Ok(())
                }),
        )
        .add_cmd(
                Command::new("purge-community-ceremony")
                    .description("purge all history within the provided ceremony index range for the specified community")
                    .options(|app| {
                        app.setting(AppSettings::ColoredHelp)
                            .from_cindex_arg()
                            .to_cindex_arg()

                    })
                .runner(|_args: &str, matches: &ArgMatches<'_>| {
                    let sudoer = AccountKeyring::Alice.pair();
                    let api = get_chain_api(matches).set_signer(sudoer);

                    let current_ceremony_index = get_ceremony_index(&api);

                    let from_cindex_arg = matches.from_cindex_arg().unwrap_or(0);
                    let to_cindex_arg = matches.to_cindex_arg().unwrap_or(0);

                    let from_cindex = into_effective_cindex(from_cindex_arg, current_ceremony_index);
                    let to_cindex = into_effective_cindex(to_cindex_arg, current_ceremony_index);

                    if from_cindex > to_cindex {
                        panic!("'from' <= 'to' ceremony index violated");
                    }
                    let cid = verify_cid(&api,
                                         matches
                                             .cid_arg()
                                             .expect("please supply argument --cid"),
                    );
                    println!("purging ceremony index range [{}  {}] for community {}", from_cindex, to_cindex, cid);

                    let calls: Vec<_> = (from_cindex..=to_cindex)
                        .map(|idx| compose_call!(
                            api.metadata,
                            "EncointerCeremonies",
                            "purge_community_ceremony",
                            (cid, idx)
                        ))
                        .collect();
                    let batch_call = compose_call!(
                        api.metadata,
                        "Utility",
                        "batch",
                        calls
                    );
                    let unsigned_sudo_call = compose_call!(
                        api.metadata,
                        "Sudo",
                        "sudo",
                        batch_call.clone()
                    );
                    info!("raw sudo batch call to sign with js/apps {}: 0x{}", cid, hex::encode(unsigned_sudo_call.encode()));
                    let xt: UncheckedExtrinsicV4<_> = compose_extrinsic!(
                        api,
                        "Sudo",
                        "sudo",
                        batch_call
                    );
                    ensure_payment(&api, &xt.hex_encode());
                    let tx_hash = api.send_extrinsic(xt.hex_encode(), XtStatus::InBlock).unwrap();
                    info!("[+] Transaction got included. Hash: {:?}\n", tx_hash);
                    Ok(())
                }),
        )
        .add_cmd(
            Command::new("set-meetup-time-offset")
                .description("signed value to offset the ceremony meetup time relative to solar noon")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                        .setting(AppSettings::AllowLeadingHyphen)
                        .time_offset_arg()
                })
                .runner(|_args: &str, matches: &ArgMatches<'_>| {
                    let api = get_chain_api(matches)
                        .set_signer(AccountKeyring::Alice.pair());
                    let time_offset = matches.time_offset_arg().unwrap_or(0);
                    let call = compose_call!(
                        &api.metadata,
                        "EncointerCeremonies",
                        "set_meetup_time_offset",
                        time_offset
                    );

                    // return calls as `OpaqueCall`s to get the same return type in both branches
                    let privileged_call = if contains_sudo_pallet(&api.metadata) {
                        let sudo_call = sudo_call(&api.metadata, call);
                        info!("Printing raw sudo call for js/apps:");
                        print_raw_call("sudo(...)", &sudo_call);
                        OpaqueCall::from_tuple(&sudo_call)
                    } else {
                        let threshold = (get_councillors(&api).unwrap().len() / 2 + 1) as u32;
                        info!("Printing raw collective propose calls with threshold {} for js/apps", threshold);
                        let propose_call = collective_propose_call(&api.metadata, threshold, call);
                        print_raw_call("collective_propose(...)", &propose_call);
                        OpaqueCall::from_tuple(&propose_call)
                    };

                    send_and_wait_for_in_block(&api, xt(&api, privileged_call));
                    Ok(())
                }),
        )
        // To handle when no subcommands match
        .no_cmd(|_args, _matches| {
            println!("No subcommand matched");
            Ok(())
        })
        .run();
}

fn get_chain_api(matches: &ArgMatches<'_>) -> Api<sr25519::Pair, WsRpcClient> {
	let url = format!(
		"{}:{}",
		matches.value_of("node-url").unwrap(),
		matches.value_of("node-port").unwrap()
	);
	debug!("connecting to {}", url);
	let client = WsRpcClient::new(&url);
	Api::<sr25519::Pair, _>::new(client).unwrap()
}

fn reasonable_native_balance(api: &Api<sr25519::Pair, WsRpcClient>) -> u128 {
	let xt = api.balance_transfer(GenericAddress::Id(AccountKeyring::Alice.into()), 9999);
	let fee = api
		.get_fee_details(&xt.hex_encode(), None)
		.unwrap()
		.unwrap()
		.inclusion_fee
		.unwrap()
		.base_fee;
	let ed = api.get_existential_deposit().unwrap();
	return ed + fee * PREFUNDING_NR_OF_TRANSFER_EXTRINSICS
}

fn listen(matches: &ArgMatches<'_>) {
	let api = get_chain_api(matches);
	debug!("Subscribing to events");
	let (events_in, events_out) = channel();
	let mut count = 0u32;
	let mut blocks = 0u32;
	api.subscribe_events(events_in.clone()).unwrap();
	loop {
		if matches.is_present("events") &&
			count >= value_t!(matches.value_of("events"), u32).unwrap()
		{
			return
		};
		if matches.is_present("blocks") &&
			blocks >= 1 + value_t!(matches.value_of("blocks"), u32).unwrap()
		{
			return
		};
		let event_str = events_out.recv().unwrap();
		let _unhex = Vec::from_hex(event_str).unwrap();
		let mut _er_enc = _unhex.as_slice();
		let _events = Vec::<frame_system::EventRecord<Event, Hash>>::decode(&mut _er_enc);
		blocks += 1;
		match _events {
			Ok(evts) =>
				for evr in &evts {
					debug!("decoded: phase {:?} event {:?}", evr.phase, evr.event);
					match &evr.event {
						Event::EncointerCeremonies(ee) => {
							count += 1;
							info!(">>>>>>>>>> ceremony event: {:?}", ee);
							match &ee {
								pallet_encointer_ceremonies::Event::ParticipantRegistered(
									cid,
									participant_type,
									accountid,
								) => {
									println!(
										"Participant registered as {:?}, for cid: {:?}, account: {}, ",
										participant_type, cid, accountid
									);
								},
								_ => println!("Unsupported EncointerCommunities event"),
							}
						},
						Event::EncointerScheduler(ee) => {
							count += 1;
							info!(">>>>>>>>>> scheduler event: {:?}", ee);
							match &ee {
								pallet_encointer_scheduler::Event::PhaseChangedTo(phase) => {
									println!("Phase changed to: {:?}", phase);
								},
								pallet_encointer_scheduler::Event::CeremonySchedulePushedByOneDay => {
									println!("Ceremony schedule was pushed by one day");
								},
							}
						},
						Event::EncointerCommunities(ee) => {
							count += 1;
							info!(">>>>>>>>>> community event: {:?}", ee);
							match &ee {
								pallet_encointer_communities::Event::CommunityRegistered(cid) => {
									println!("Community registered: cid: {:?}", cid);
								},
								pallet_encointer_communities::Event::MetadataUpdated(cid) => {
									println!("Community metadata updated cid: {:?}", cid);
								},
								pallet_encointer_communities::Event::NominalIncomeUpdated(
									cid,
									income,
								) => {
									println!(
										"Community metadata updated cid: {:?}, value: {:?}",
										cid, income
									);
								},
								pallet_encointer_communities::Event::DemurrageUpdated(
									cid,
									demurrage,
								) => {
									println!(
										"Community metadata updated cid: {:?}, value: {:?}",
										cid, demurrage
									);
								},
								_ => println!("Unsupported EncointerCommunities event"),
							}
						},
						Event::EncointerBalances(ee) => {
							count += 1;
							println!(">>>>>>>>>> encointer balances event: {:?}", ee);
						},
						Event::EncointerBazaar(ee) => {
							count += 1;
							println!(">>>>>>>>>> encointer bazaar event: {:?}", ee);
						},
						Event::System(ee) => match ee {
							frame_system::Event::ExtrinsicFailed {
								dispatch_error,
								dispatch_info,
							} => {
								error!("ExtrinsicFailed: {:?} {:?}", dispatch_error, dispatch_info);
							},
							frame_system::Event::ExtrinsicSuccess { dispatch_info } => {
								println!("ExtrinsicSuccess: {:?}", dispatch_info);
							},
							_ => debug!("ignoring unsupported system Event"),
						},
						_ => debug!("ignoring unsupported module event: {:?}", evr.event),
					}
				},
			Err(_) => error!("couldn't decode event record list"),
		}
	}
}

/// Extracts api and cid from `matches` and execute the given `closure` with them.
fn extract_and_execute<T>(
	matches: &ArgMatches<'_>,
	closure: impl FnOnce(Api<sr25519::Pair, WsRpcClient>, CommunityIdentifier) -> T,
) -> T {
	let api = get_chain_api(matches);
	let cid = verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"));
	closure(api, cid)
}

fn verify_cid(api: &Api<sr25519::Pair, WsRpcClient>, cid: &str) -> CommunityIdentifier {
	let cids = get_community_identifiers(&api).expect("no community registered");
	let cid = CommunityIdentifier::from_str(cid).unwrap();
	if !cids.contains(&cid) {
		panic!("cid {} does not exist on chain", cid);
	}
	cid
}

fn get_block_number(api: &Api<sr25519::Pair, WsRpcClient>) -> BlockNumber {
	let hdr: Header = api.get_header(None).unwrap().unwrap();
	debug!("decoded: {:?}", hdr);
	//let hdr: Header= Decode::decode(&mut .as_bytes()).unwrap();
	hdr.number
}

fn get_demurrage_per_block(
	api: &Api<sr25519::Pair, WsRpcClient>,
	cid: CommunityIdentifier,
) -> Demurrage {
	let d: Option<Demurrage> = api
		.get_storage_map("EncointerBalances", "DemurragePerBlock", cid, None)
		.unwrap();

	match d {
		Some(d) => {
			debug!("Fetched community specific demurrage per block {:?}", &d);
			d
		},
		None => {
			let d = api.get_constant("EncointerBalances", "DefaultDemurrage").unwrap();
			debug!("Fetched default demurrage per block {:?}", d);
			d
		},
	}
}

fn get_ceremony_index(api: &Api<sr25519::Pair, WsRpcClient>) -> CeremonyIndexType {
	api.get_storage_value("EncointerScheduler", "CurrentCeremonyIndex", None)
		.unwrap()
		.unwrap()
}

fn get_attestee_count(
	api: &Api<sr25519::Pair, WsRpcClient>,
	key: CommunityCeremony,
) -> ParticipantIndexType {
	api.get_storage_map("EncointerCeremonies", "AttestationCount", key, None)
		.unwrap()
		.or(Some(0))
		.unwrap()
}

fn get_attestees(
	api: &Api<sr25519::Pair, WsRpcClient>,
	key: CommunityCeremony,
	windex: ParticipantIndexType,
) -> Option<Vec<AccountId>> {
	api.get_storage_double_map("EncointerCeremonies", "AttestationRegistry", key, windex, None)
		.unwrap()
}

fn get_participant_attestation_index(
	api: &Api<sr25519::Pair, WsRpcClient>,
	key: CommunityCeremony,
	accountid: &AccountId,
) -> Option<ParticipantIndexType> {
	api.get_storage_double_map("EncointerCeremonies", "AttestationIndex", key, accountid, None)
		.unwrap()
}

fn new_claim_for(
	api: &Api<sr25519::Pair, WsRpcClient>,
	claimant: &sr25519::Pair,
	cid: CommunityIdentifier,
	n_participants: u32,
) -> Vec<u8> {
	let cindex = get_ceremony_index(api);
	let mindex = api
		.get_meetup_index(&(cid, cindex), &claimant.public().into())
		.unwrap()
		.expect("participant must be assigned to meetup to generate a claim");

	// implicitly assume that participant meet at the right place at the right time
	let mloc = api.get_meetup_location(&(cid, cindex), mindex).unwrap().unwrap();
	let mtime = api.get_meetup_time(mloc, ONE_DAY).unwrap();

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

fn get_community_identifiers(
	api: &Api<sr25519::Pair, WsRpcClient>,
) -> Option<Vec<CommunityIdentifier>> {
	api.get_storage_value("EncointerCommunities", "CommunityIdentifiers", None)
		.unwrap()
}

/// This rpc needs to have offchain indexing enabled in the node.
fn get_cid_names(api: &Api<sr25519::Pair, WsRpcClient>) -> Option<Vec<CidName>> {
	let req = json!({
		"method": "encointer_getAllCommunities",
		"params": [],
		"jsonrpc": "2.0",
		"id": "1",
	});

	let n = api.get_request(req.into()).unwrap().expect(
		"No communities returned. Are you running the node with `--enable-offchain-indexing true`?",
	);
	Some(serde_json::from_str(&n).unwrap())
}

fn get_businesses(
	api: &Api<sr25519::Pair, WsRpcClient>,
	cid: CommunityIdentifier,
) -> Option<Vec<BusinessData>> {
	let req = json!({
		"method": "encointer_bazaarGetBusinesses",
		"params": vec![cid],
		"jsonrpc": "2.0",
		"id": "1",
	});

	let n = api.get_request(req.into()).unwrap().expect("Could not find any businesses...");
	Some(serde_json::from_str(&n).unwrap())
}

fn get_offerings(
	api: &Api<sr25519::Pair, WsRpcClient>,
	cid: CommunityIdentifier,
) -> Option<Vec<OfferingData>> {
	let req = json!({
		"method": "encointer_bazaarGetOfferings",
		"params": vec![cid],
		"jsonrpc": "2.0",
		"id": "1",
	});

	let n = api
		.get_request(req.into())
		.unwrap()
		.expect("Could not find any business offerings...");
	Some(serde_json::from_str(&n).unwrap())
}

fn get_offerings_for_business(
	api: &Api<sr25519::Pair, WsRpcClient>,
	cid: CommunityIdentifier,
	account_id: AccountId,
) -> Option<Vec<OfferingData>> {
	let b_id = BusinessIdentifier::new(cid, account_id);

	let req = json!({
		"method": "encointer_bazaarGetOfferingsForBusiness",
		"params": vec![to_value(b_id).unwrap()],
		"jsonrpc": "2.0",
		"id": "1",
	});

	let n = api
		.get_request(req.into())
		.unwrap()
		.expect("Could not find any business offerings...");
	Some(serde_json::from_str(&n).unwrap())
}

fn get_reputation_history(
	api: &Api<sr25519::Pair, WsRpcClient>,
	account_id: &AccountId,
) -> Option<Vec<(CeremonyIndexType, CommunityReputation)>> {
	let req = json!({
		"method": "encointer_getReputations",
		"params": vec![account_id],
		"jsonrpc": "2.0",
		"id": "1",
	});

	let n = api
		.get_request(req.into())
		.unwrap()
		.expect("Could not query reputation history...");
	Some(serde_json::from_str(&n).unwrap())
}

fn get_all_balances(
	api: &Api<sr25519::Pair, WsRpcClient>,
	account_id: &AccountId,
) -> Option<Vec<(CommunityIdentifier, BalanceEntry<BlockNumber>)>> {
	let req = json!({
		"method": "encointer_getAllBalances",
		"params": vec![account_id],
		"jsonrpc": "2.0",
		"id": "1",
	});

	let n = api.get_request(req.into()).unwrap().unwrap(); //expect("Could not query all balances...");

	serde_json::from_str(&n).ok()
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

fn get_reputation(
	api: &Api<sr25519::Pair, WsRpcClient>,
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
	.unwrap()
	.or(Some(Reputation::Unverified))
	.unwrap()
}

fn apply_demurrage(
	entry: BalanceEntry<BlockNumber>,
	current_block: BlockNumber,
	demurrage_per_block: BalanceType,
) -> BalanceType {
	let elapsed_time_block_number = current_block.checked_sub(entry.last_update).unwrap();
	let elapsed_time_u32: u32 = elapsed_time_block_number.try_into().unwrap();
	let elapsed_time = BalanceType::from_num(elapsed_time_u32);
	let exponent: BalanceType = -demurrage_per_block * elapsed_time;
	debug!(
		"demurrage per block {}, current_block {}, last {}, elapsed_blocks {}",
		demurrage_per_block, current_block, entry.last_update, elapsed_time
	);
	let exp_result: BalanceType = exp(exponent).unwrap();
	entry.principal.checked_mul(exp_result).unwrap()
}

fn send_bazaar_xt(matches: &ArgMatches<'_>, business_call: &BazaarCalls) -> Result<(), ()> {
	let business_owner = matches.account_arg().map(get_pair_from_str).unwrap();

	let api = get_chain_api(matches).set_signer(business_owner.clone().into());
	let cid = verify_cid(&api, matches.cid_arg().expect("please supply argument --cid"));
	let ipfs_cid = matches.ipfs_cid_arg().expect("ipfs cid needed");

	let xt: UncheckedExtrinsicV4<_> = compose_extrinsic!(
		api.clone(),
		"EncointerBazaar",
		&business_call.to_string(),
		cid,
		ipfs_cid
	);
	ensure_payment(&api, &xt.hex_encode());
	// send and watch extrinsic until finalized
	let _ = api.send_extrinsic(xt.hex_encode(), XtStatus::Ready).unwrap();
	println!("Creating business for {}. xt-status: 'ready'", business_owner.public());
	Ok(())
}

fn endorse_newcomers(
	api: &mut Api<sr25519::Pair, WsRpcClient>,
	cid: CommunityIdentifier,
	matches: &ArgMatches<'_>,
) -> Result<(), ApiClientError> {
	let bootstrapper = matches.bootstrapper_arg().map(get_pair_from_str).unwrap();
	let endorsees = matches.endorsees_arg().expect("Please supply at least one endorsee");

	api.signer = Some(bootstrapper.into());

	let mut nonce = api.get_nonce()?;

	for e in endorsees.into_iter() {
		let endorsee = get_accountid_from_str(e);

		let call =
			compose_call!(api.metadata, "EncointerCeremonies", "endorse_newcomer", cid, endorsee);

		let xt = offline_xt(&api, call, nonce);

		ensure_payment(&api, &xt.hex_encode());

		let _tx_hash = api.send_extrinsic(xt.hex_encode(), XtStatus::Ready).unwrap();

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

fn get_bootstrappers_with_remaining_newbie_tickets(
	api: &Api<sr25519::Pair, WsRpcClient>,
	cid: CommunityIdentifier,
) -> Result<Vec<BootstrapperWithTickets>, ApiClientError> {
	let total_newbie_tickets: u8 = api
		.get_storage_value("EncointerCeremonies", "EndorsementTicketsPerBootstrapper", None)
		.unwrap()
		.unwrap();

	// prepare closure to make below call more readable.
	let ticket_query = |bs| -> Result<u8, ApiClientError> {
		let remaining_tickets = total_newbie_tickets -
			api.get_storage_double_map(
				"EncointerCeremonies",
				"BurnedBootstrapperNewbieTickets",
				cid,
				bs,
				None,
			)?
			.unwrap_or(0u8);

		Ok(remaining_tickets)
	};

	let bootstrappers: Vec<AccountId> = api
		.get_storage_map("EncointerCommunities", "Bootstrappers", cid, None)?
		.expect("No bootstrappers found, does the community exist?");

	let mut bs_with_tickets: Vec<BootstrapperWithTickets> = Vec::with_capacity(bootstrappers.len());

	for bs in bootstrappers.into_iter() {
		bs_with_tickets.push(BootstrapperWithTickets {
			bootstrapper: bs.clone(),
			remaining_newbie_tickets: ticket_query(bs)?,
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
