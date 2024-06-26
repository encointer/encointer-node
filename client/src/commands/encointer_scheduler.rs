use crate::{
	cli_args::EncointerArgsExtractor,
	commands::frame::get_block_number,
	utils::{
		collective_propose_call, contains_sudo_pallet, get_chain_api, get_councillors,
		keys::get_pair_from_str, print_raw_call, send_and_wait_for_in_block, sudo_call, xt,
		OpaqueCall,
	},
};
use clap::ArgMatches;
use encointer_api_client_extension::{
	set_api_extrisic_params_builder, ParentchainExtrinsicSigner, SchedulerApi,
};
use encointer_node_notee_runtime::Moment;

use log::{debug, info};

use sp_keyring::AccountKeyring;
use substrate_api_client::ac_compose_macros::compose_call;

pub fn get_phase(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;

		// >>>> add some debug info as well
		let bn = get_block_number(&api, None).await;
		debug!("block number: {}", bn);
		let cindex = api.get_ceremony_index(None).await;
		info!("ceremony index: {}", cindex);
		let tnext: Moment = api.get_next_phase_timestamp(None).await.unwrap();
		debug!("next phase timestamp: {}", tnext);
		// <<<<

		let phase = api.get_current_phase(None).await.unwrap();
		println!("{phase:?}");
		Ok(())
	})
	.into()
}

pub fn get_cindex(_args: &str, matches: &ArgMatches<'_>) -> Result<(), clap::Error> {
	let rt = tokio::runtime::Runtime::new().unwrap();
	rt.block_on(async {
		let api = get_chain_api(matches).await;

		// >>>> add some debug info as well
		let bn = get_block_number(&api, None).await;
		debug!("block number: {}", bn);
		let cindex = api.get_ceremony_index(None).await;
		info!("ceremony index: {}", cindex);
		println!("{cindex}");
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

		let phase = api.get_current_phase(None).await.unwrap();
		println!("Phase is now: {phase:?}");
		Ok(())
	})
	.into()
}
