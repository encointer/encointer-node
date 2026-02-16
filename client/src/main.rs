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
mod commands;
mod community_spec;
mod utils;

use clap::{AppSettings, Arg};
use clap_nested::{Command, Commander};
use cli_args::EncointerArgs;

use encointer_node_notee_runtime::BalanceType;

const PREFUNDING_NR_OF_TRANSFER_EXTRINSICS: u128 = 1000;
const VERSION: &str = env!("CARGO_PKG_VERSION");

mod exit_code {
	pub const WRONG_PHASE: i32 = 50;
	pub const FEE_PAYMENT_FAILED: i32 = 51;
	pub const INVALID_REPUTATION: i32 = 52;
	pub const RPC_ERROR: i32 = 60;
	pub const NOT_CC_HOLDER: i32 = 61;
	pub const NO_CID_SPECIFIED: i32 = 70;
}

fn main() {
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
		.add_cmd(
		    Command::new("new-account")
		        .description("Imports account into the key store. Either creates a new account or with the supplied seed.")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .seed_arg()
		        })
		        .runner(commands::keystore::new_account),
		)
		.add_cmd(
		    Command::new("export-secret")
		        .description("prints the mnemonic phrase for an account in the keystore")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
						.arg(
							Arg::with_name("account")
								.takes_value(true)
								.required(true)
								.value_name("SS58")
								.help("AccountId to be exported"),
						)
		        })
		        .runner(commands::keystore::export_secret),
		)
		.add_cmd(
		    Command::new("list-accounts")
		        .description("lists all accounts in keystore")
		        .runner(commands::keystore::list_accounts),
		)
		.add_cmd(
		    Command::new("print-metadata")
		        .description("query node metadata and print it as json to stdout")
		        .runner(commands::frame::print_metadata),
		)
		.add_cmd(
		    Command::new("faucet")
		        .description("send some bootstrapping funds to supplied account(s)")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		            .fundees_arg()
		        })
		        .runner(commands::frame::faucet),
		)
		.add_cmd(
		    Command::new("balance")
		        .description("query on-chain balance for AccountId. If --cid is supplied, returns balance in that community. Otherwise balance of native ERT token")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		            .account_arg()
		            .all_flag()
		            .at_block_arg()
		        })
		        .runner(commands::encointer_core::balance),
		)
		.add_cmd(
		    Command::new("issuance")
		        .description("query total issuance for community. must supply --cid")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		            .at_block_arg()
		        })
		        .runner(commands::encointer_core::issuance),
		)
		.add_cmd(
		    Command::new("transfer")
		        .description("transfer funds from one account to another. If --cid is supplied, send that community (amount is fixpoint). Otherwise send native ERT tokens (amount is integer)")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		            .dryrun_flag()
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
		        .runner(commands::encointer_core::transfer),
		)
		.add_cmd(
		    Command::new("transfer_all")
		        .description("transfer all available funds from one account to another for a community specified with --cid.")
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
		        })
		        .runner(commands::encointer_core::transfer_all),
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
		        .runner(commands::encointer_core::listen_to_events),
		)
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
		            .signer_arg("account with necessary privileges")
						.dryrun_flag()
						.wrap_call_arg()
						.batch_size_arg()
		        })
		        .runner(commands::encointer_communities::new_community),
		)
		.add_cmd(
		    Command::new("add-locations")
		        .description("Register new locations for a community")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .signer_arg("account with necessary privileges")
		                .dryrun_flag()
		                .arg(
		                    Arg::with_name("specfile")
		                        .takes_value(true)
		                        .required(true)
		                        .help("geojson file that specifies locations to add as points"),
		                )
		        })
		        .runner(commands::encointer_communities::add_locations),
		)
		.add_cmd(
			Command::new("remove-location")
				.description("Remove a location a for a community. Check polkadot-js/apps to find the geohash")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.signer_arg("account with necessary privileges")
						.dryrun_flag()
						.optional_cid_arg()
						.geohash_arg()
						.location_index_arg()
				})
				.runner(commands::encointer_communities::remove_locations),
		)
		.add_cmd(
		    Command::new("list-communities")
		        .description("list all registered communities")
				.options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .at_block_arg()
		        })
		        .runner(commands::encointer_communities::list_communities),
		)
		.add_cmd(
		    Command::new("list-locations")
		        .description("list all meetup locations for a community")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .at_block_arg()
		        })
		        .runner(commands::encointer_communities::list_locations),
		)
		.add_cmd(
		    Command::new("get-phase")
		        .description("read current ceremony phase from chain")
		        .runner(commands::encointer_scheduler::get_phase),
		)
		.add_cmd(
			Command::new("get-cindex")
				.description("read current ceremony index from chain")
				.runner(commands::encointer_scheduler::get_cindex),
		)
		.add_cmd(
		    Command::new("next-phase")
		        .description("Advance ceremony state machine to next phase by ROOT call")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .signer_arg("account with necessary privileges (sudo or councillor)")
		        })
		       .runner(commands::encointer_scheduler::next_phase),
		)
		.add_cmd(
		    Command::new("list-participants")
		        .description("list all registered participants supplied community identifier and ceremony index")
		        .options(|app| {
		        app.setting(AppSettings::ColoredHelp).setting(AppSettings::AllowNegativeNumbers)
		            .ceremony_index_arg()
					.at_block_arg()
		        })
		        .runner(commands::encointer_ceremonies::list_participants),
		)
		.add_cmd(
		    Command::new("list-meetups")
		        .description("list all assigned meetups for supplied community identifier and ceremony index")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp).setting(AppSettings::AllowNegativeNumbers)
		                .ceremony_index_arg()
						.at_block_arg()
		        })
		        .runner(commands::encointer_ceremonies::list_meetups),
		)
		.add_cmd(
		    Command::new("print-ceremony-stats")
		        .description("pretty prints all information for a community ceremony")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp).setting(AppSettings::AllowNegativeNumbers)
		                .ceremony_index_arg()
						.at_block_arg()
		        })
		        .runner(commands::encointer_ceremonies::print_ceremony_stats),
		)
		.add_cmd(
		    Command::new("list-attestees")
		        .description("list all attestees for participants for supplied community identifier and ceremony index")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp).setting(AppSettings::AllowNegativeNumbers)
		                .ceremony_index_arg()
						.at_block_arg()
		        })
		        .runner(commands::encointer_ceremonies::list_attestees),
		)
		.add_cmd(
		    Command::new("list-reputables")
		        .description("list all reputables for all cycles within the current reputation-lifetime for all communities")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .at_block_arg()
		                .verbose_flag()
		        })
		        .runner(commands::encointer_ceremonies::list_reputables),
		        )
		.add_cmd(
		    Command::new("register-participant")
		        .description("Register encointer ceremony participant for supplied community")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		            .account_arg()
		            .signer_arg("Account which signs the tx.")
		        })
		        .runner(commands::encointer_ceremonies::register_participant),
		)
		.add_cmd(
		    Command::new("upgrade-registration")
		        .description("Upgrade registration to repuable for encointer ceremony participant for supplied community")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		            .account_arg()
		            .signer_arg("Account which signs the tx.")
		        })
		        .runner(commands::encointer_ceremonies::upgrade_registration),
		)
		.add_cmd(
		    Command::new("unregister-participant")
		        .description("Unregister encointer ceremony participant for supplied community")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp).setting(AppSettings::AllowNegativeNumbers)
		            .account_arg()
		            .signer_arg("Account which signs the tx.")
		            .ceremony_index_arg()
		        })
		        .runner(commands::encointer_ceremonies::unregister_participant),
		)
		.add_cmd(
		    Command::new("endorse-newcomers")
		        .description("Endorse newbies with a bootstrapper account")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .bootstrapper_arg()
		                .endorsees_arg()
		        })
		        .runner(commands::encointer_ceremonies::endorse),
		)
		.add_cmd(
		    Command::new("get-bootstrappers-with-remaining-newbie-tickets")
		        .description("Get the bootstrappers along with the remaining newbie tickets")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		        })
		       .runner(commands::encointer_ceremonies::bootstrappers_with_remaining_newbie_tickets),
		)
		.add_cmd(
		    Command::new("get-proof-of-attendance")
		        .description("creates a proof of ProofOfAttendances for an <account> for the given ceremony index")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .setting(AppSettings::AllowNegativeNumbers)
		                .account_arg()
		                .ceremony_index_arg()
		        })
		        .runner(commands::encointer_ceremonies::get_proof_of_attendance),
		)
		.add_cmd(
		    Command::new("attest-attendees")
		        .description("Register encointer ceremony claim of attendances for supplied community")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		                .optional_cid_arg()
		                .attestees_arg()
		        })
		        .runner(commands::encointer_ceremonies::attest_attendees),
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
		        .runner(commands::encointer_ceremonies::new_claim),
		)
		.add_cmd(
		    Command::new("claim-reward")
		        .description("Claim the rewards for all meetup participants of the last ceremony.")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .signer_arg("Account which signs the tx.")
		                .meetup_index_arg()
		                .all_flag()
		        })
		        .runner(commands::encointer_ceremonies::claim_reward),
		)
		.add_cmd(
		    Command::new("reputation")
		        .description("List reputation history for an account")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()})
		        .runner(commands::encointer_ceremonies::reputation),
		)
		.add_cmd(
			Command::new("purge-community-ceremony")
				.description("purge all history within the provided ceremony index range for the specified community")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.from_cindex_arg()
						.to_cindex_arg()

				})
				.runner(commands::encointer_ceremonies::purge_community_ceremony),
		)
		.add_cmd(
			Command::new("set-meetup-time-offset")
				.description("signed value to offset the ceremony meetup time relative to solar noon")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.setting(AppSettings::AllowLeadingHyphen)
						.time_offset_arg()
				})
				.runner(commands::encointer_ceremonies::set_meetup_time_offset),
		)
		.add_cmd(
		    Command::new("create-business")
		        .description("Register a community business on behalf of the account")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		                .ipfs_cid_arg()
		        })
		        .runner(commands::encointer_bazaar::create_business),
		)
		.add_cmd(
		    Command::new("update-business")
		        .description("Update an already existing community business on behalf of the account")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		                .ipfs_cid_arg()
		        })
		        .runner(commands::encointer_bazaar::update_business),
		)
		.add_cmd(
		    Command::new("create-offering")
		        .description("Create an offering for the business belonging to account")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		                .ipfs_cid_arg()
		        })
		        .runner(commands::encointer_bazaar::create_offering),
		)
		.add_cmd(
		    Command::new("list-businesses")
		        .description("List businesses for a community")
		        .runner(commands::encointer_bazaar::list_businesses),
		)
		.add_cmd(
		    Command::new("list-offerings")
		        .description("List offerings for a community")
		        .runner(commands::encointer_bazaar::list_offerings),
		)
		.add_cmd(
		    Command::new("list-business-offerings")
		        .description("List offerings for a business")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		        })
		        .runner(commands::encointer_bazaar::list_business_offerings),
		)
		.add_cmd(
		    Command::new("ipfs-upload")
		        .description("Upload file to IPFS via authenticated gateway (requires CC holder)")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .signer_arg("account to authenticate (must be CC holder)")
		                .optional_cid_arg()
		                .gateway_url_arg()
		                .file_path_arg()
		        })
		        .runner(commands::encointer_ipfs::ipfs_upload),
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
		        .runner(commands::encointer_faucet::create_faucet),
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
		        .runner(commands::encointer_faucet::drip_faucet),
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
		       .runner(commands::encointer_faucet::dissolve_faucet),
		)
		.add_cmd(
		    Command::new("close-faucet")
		        .description("lazy garbage collection. can only be called by faucet creator and only once the faucet is empty")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .account_arg()
		                .faucet_account_arg()
		        })
		        .runner(commands::encointer_faucet::close_faucet),
		)
		.add_cmd(
		    Command::new("set-faucet-reserve-amount")
		        .description("Set faucet pallet reserve amount")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .signer_arg("account with necessary privileges (sudo or councillor)")
		                .faucet_reserve_amount_arg()
		        })
		       .runner(commands::encointer_faucet::set_faucet_reserve_amount),
		)
		.add_cmd(
		    Command::new("list-faucets")
		        .description("list all faucets. use -v to get faucet details.")
		        .options(|app| {
		            app.setting(AppSettings::ColoredHelp)
		                .at_block_arg()
		                .verbose_flag()
		        })
		       .runner(commands::encointer_faucet::list_faucets)
		)
		.add_cmd(
			Command::new("list-commitments")
				.description("list all reputation commitments")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.purpose_id_arg()
						.at_block_arg()
				})
				.runner(commands::encointer_reputation_commitments::list_commitments)
		)
		.add_cmd(
			Command::new("list-purposes")
				.description("list all reputation commitment purpose descriptors")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.at_block_arg()
				})
				.runner(commands::encointer_reputation_commitments::list_purposes)
		)
		.add_cmd(
			Command::new("register-bandersnatch-key")
				.description("Register a Bandersnatch public key for reputation rings")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.account_arg()
						.arg(
							Arg::with_name("key")
								.long("key")
								.takes_value(true)
								.required(true)
								.value_name("HEX")
								.help("hex-encoded 32-byte Bandersnatch public key"),
						)
				})
				.runner(commands::encointer_reputation_rings::register_bandersnatch_key),
		)
		.add_cmd(
			Command::new("initiate-rings")
				.description("Initiate ring computation for a community at a ceremony index")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.account_arg()
						.arg(
							Arg::with_name("ceremony-index")
								.long("ceremony-index")
								.takes_value(true)
								.required(true)
								.value_name("U32")
								.help("ceremony index for which to compute rings"),
						)
				})
				.runner(commands::encointer_reputation_rings::initiate_rings),
		)
		.add_cmd(
			Command::new("continue-ring-computation")
				.description("Continue the pending ring computation (one step)")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.account_arg()
				})
				.runner(commands::encointer_reputation_rings::continue_ring_computation),
		)
		.add_cmd(
			Command::new("get-rings")
				.description("Query ring members for a community and ceremony index")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.at_block_arg()
						.arg(
							Arg::with_name("ceremony-index")
								.long("ceremony-index")
								.takes_value(true)
								.required(true)
								.value_name("U32")
								.help("ceremony index to query rings for"),
						)
				})
				.runner(commands::encointer_reputation_rings::get_rings),
		)
		.add_cmd(
			Command::new("submit-set-inactivity-timeout-proposal")
				.description("Submit set inactivity timeout proposal")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.account_arg()
						.inactivity_timeout_arg()
				})
				.runner(commands::encointer_democracy::submit_set_inactivity_timeout_proposal),
		)
		.add_cmd(
			Command::new("submit-update-nominal-income-proposal")
				.description("Submit update nominal income proposal for specified community")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.account_arg()
						.nominal_income_arg()
				})
				.runner(commands::encointer_democracy::submit_update_nominal_income_proposal),
		)
		.add_cmd(
			Command::new("submit-update-demurrage-proposal")
				.description("Submit update demurrage proposal for specified community")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.account_arg()
						.demurrage_halving_blocks_arg()
				})
				.runner(commands::encointer_democracy::submit_update_demurrage_proposal),
		)
		.add_cmd(
			Command::new("submit-petition")
				.description("Submit a petition for specified community (if --cid specified) or global")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.account_arg()
						.arg(
							Arg::with_name("demand")
								.takes_value(true)
								.required(true)
								.value_name("DEMAND")
								.help("what the petition demands"),
						)
				})
				.runner(commands::encointer_democracy::submit_petition),
		)
		.add_cmd(
			Command::new("submit-spend-native-proposal")
				.description("Submit 'spend native' proposal for specified community, amount and beneficiary")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.account_arg()
						.arg(
							Arg::with_name("to")
								.takes_value(true)
								.required(true)
								.value_name("SS58")
								.help("beneficiary's AccountId in ss58check format"),
						)
						.arg(
							Arg::with_name("amount")
								.takes_value(true)
								.required(true)
								.value_name("U128")
								.help("amount to be transferred"),
						)
				})
				.runner(commands::encointer_democracy::submit_spend_native_proposal),
		)
		.add_cmd(
			Command::new("list-proposals")
				.description("list all proposals.")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.at_block_arg()
						.all_flag()
				})
			   .runner(commands::encointer_democracy::list_proposals),
				)
        .add_cmd(
            Command::new("list-enactment-queue")
                .description("list queued proposal enactments")
                .options(|app| {
                    app.setting(AppSettings::ColoredHelp)
                        .at_block_arg()
                })
                .runner(commands::encointer_democracy::list_enactment_queue),
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
				.runner(commands::encointer_democracy::vote),
		)
		.add_cmd(
			Command::new("update-proposal-state")
				.description("Update proposal state")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp).account_arg().proposal_id_arg()
				})
				.runner(commands::encointer_democracy::update_proposal_state),
		)
		.add_cmd(
			Command::new("get-treasury")
				.description("get treasury address for a community")
				.runner(commands::encointer_treasuries::get_treasury_account),
		)
		.add_cmd(
			Command::new("register-offline-identity")
				.description("Register an offline payment identity (ZK commitment) for an account")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.account_arg()
				})
				.runner(commands::encointer_offline_payment::register_offline_identity),
		)
		.add_cmd(
			Command::new("get-offline-identity")
				.description("Get the offline payment identity (commitment) for an account")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.account_arg()
						.at_block_arg()
				})
				.runner(commands::encointer_offline_payment::get_offline_identity),
		)
		.add_cmd(
			Command::new("generate-offline-payment")
				.description("Generate an offline payment proof (outputs JSON)")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.signer_arg("sender account (--signer)")
						.arg(
							Arg::with_name("to")
								.long("to")
								.takes_value(true)
								.required(true)
								.value_name("SS58")
								.help("recipient's AccountId in ss58check format"),
						)
						.arg(
							Arg::with_name("amount")
								.long("amount")
								.takes_value(true)
								.required(true)
								.value_name("FLOAT")
								.help("amount to transfer"),
						)
						.arg(
							Arg::with_name("pk-file")
								.long("pk-file")
								.takes_value(true)
								.value_name("PATH")
								.help("path to proving key file (omit for test key)"),
						)
						.optional_cid_arg()
				})
				.runner(commands::encointer_offline_payment::generate_offline_payment),
		)
		.add_cmd(
			Command::new("submit-offline-payment")
				.description("Submit an offline payment proof for settlement")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.signer_arg("account to sign the transaction")
						.arg(
							Arg::with_name("proof-file")
								.long("proof-file")
								.takes_value(true)
								.value_name("PATH")
								.help("path to JSON file containing proof (alternative to inline args)"),
						)
						.arg(
							Arg::with_name("proof")
								.long("proof")
								.takes_value(true)
								.value_name("HEX")
								.help("hex-encoded proof"),
						)
						.arg(
							Arg::with_name("sender")
								.long("sender")
								.takes_value(true)
								.value_name("SS58")
								.help("sender's AccountId"),
						)
						.arg(
							Arg::with_name("recipient")
								.long("recipient")
								.takes_value(true)
								.value_name("SS58")
								.help("recipient's AccountId"),
						)
						.arg(
							Arg::with_name("amount")
								.long("amount")
								.takes_value(true)
								.value_name("FLOAT")
								.help("transfer amount"),
						)
						.arg(
							Arg::with_name("nullifier")
								.long("nullifier")
								.takes_value(true)
								.value_name("HEX")
								.help("hex-encoded nullifier"),
						)
						.optional_cid_arg()
						.tx_payment_cid_arg()
				})
				.runner(commands::encointer_offline_payment::submit_offline_payment),
		)
		.add_cmd(
			Command::new("set-offline-payment-vk")
				.description("Set the Groth16 verification key for offline payments (requires sudo)")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.signer_arg("sudo account (defaults to //Alice)")
						.arg(
							Arg::with_name("vk-file")
								.long("vk-file")
								.takes_value(true)
								.value_name("PATH")
								.help("path to verifying key file (from generate-trusted-setup)"),
						)
						.arg(
							Arg::with_name("vk")
								.long("vk")
								.takes_value(true)
								.value_name("HEX")
								.help("hex-encoded verification key (alternative to --vk-file)"),
						)
						.tx_payment_cid_arg()
				})
				.runner(commands::encointer_offline_payment::set_verification_key),
		)
		.add_cmd(
			Command::new("generate-test-vk")
				.description("Generate and print the test verification key (hex)")
				.runner(commands::encointer_offline_payment::generate_test_vk),
		)
		.add_cmd(
			Command::new("generate-trusted-setup")
				.description(concat!(
					"Generate proving key + verifying key for offline payments.\n\n",
					"TRUSTED SETUP CEREMONY — PROCESS OVERVIEW\n",
					"==========================================\n\n",
					"The offline payment system uses Groth16 zero-knowledge proofs.\n",
					"A one-time trusted setup must be performed before the system can be used.\n",
					"The setup produces two keys:\n\n",
					"  Proving Key (PK)   — used by wallets to generate payment proofs (~50-100 KB)\n",
					"  Verifying Key (VK) — stored on-chain to verify proofs (~400-600 bytes)\n\n",
					"STEPS:\n\n",
					"  1. GENERATE: A trusted individual runs this command on a secure, air-gapped\n",
					"     machine. The OS CSPRNG provides randomness. The machine should be wiped\n",
					"     after key generation to destroy the toxic waste (internal randomness).\n\n",
					"     $ encointer-client-notee generate-trusted-setup \\\n",
					"         --pk-out proving_key.bin --vk-out verifying_key.bin\n\n",
					"  2. VERIFY: Independently verify that PK and VK are consistent:\n\n",
					"     $ encointer-client-notee verify-trusted-setup \\\n",
					"         --pk proving_key.bin --vk verifying_key.bin\n\n",
					"  3. SET ON-CHAIN: Submit the VK via governance (or sudo in dev):\n\n",
					"     $ encointer-client-notee set-offline-payment-vk \\\n",
					"         --vk-file verifying_key.bin --signer //Alice\n\n",
					"  4. DISTRIBUTE PK: Bundle proving_key.bin in the wallet app. All wallet\n",
					"     users need the PK to generate proofs. The PK is NOT secret.\n\n",
					"  5. DESTROY TOXIC WASTE: Securely wipe the machine used for generation.\n",
					"     If the internal randomness is recovered, proofs can be forged.\n\n",
					"SECURITY MODEL:\n",
					"  This is a single-party trusted setup. The generator must be trusted.\n",
					"  For higher security, consider a multi-party computation (MPC) ceremony\n",
					"  where multiple independent parties contribute randomness."
				))
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.arg(
							Arg::with_name("pk-out")
								.long("pk-out")
								.takes_value(true)
								.value_name("PATH")
								.default_value("proving_key.bin")
								.help("output path for the proving key"),
						)
						.arg(
							Arg::with_name("vk-out")
								.long("vk-out")
								.takes_value(true)
								.value_name("PATH")
								.default_value("verifying_key.bin")
								.help("output path for the verifying key"),
						)
				})
				.runner(commands::encointer_offline_payment::generate_trusted_setup),
		)
		.add_cmd(
			Command::new("verify-trusted-setup")
				.description("Verify that a proving key and verifying key are consistent")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.arg(
							Arg::with_name("pk")
								.long("pk")
								.takes_value(true)
								.required(true)
								.value_name("PATH")
								.help("path to the proving key file"),
						)
						.arg(
							Arg::with_name("vk")
								.long("vk")
								.takes_value(true)
								.required(true)
								.value_name("PATH")
								.help("path to the verifying key file"),
						)
				})
				.runner(commands::encointer_offline_payment::verify_trusted_setup),
		)
		.add_cmd(
			Command::new("inspect-setup-key")
				.description("Inspect a proving key or verifying key file (shows size, hash, type)")
				.options(|app| {
					app.setting(AppSettings::ColoredHelp)
						.arg(
							Arg::with_name("file")
								.long("file")
								.takes_value(true)
								.required(true)
								.value_name("PATH")
								.help("path to the key file to inspect"),
						)
				})
				.runner(commands::encointer_offline_payment::inspect_setup_key),
		)
		// To handle when no subcommands match
		.no_cmd(|_args, _matches| {
			println!("No subcommand matched");
			Ok(())
		})
		.run();
}
