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
		// To handle when no subcommands match
		.no_cmd(|_args, _matches| {
			println!("No subcommand matched");
			Ok(())
		})
		.run();
}
