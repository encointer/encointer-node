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
//! encointer-cli ceremony phase
//! encointer-cli transfer //Alice 5G9RtsTbiYJYQYMHbWfyPoeuuxNaCbC16tZ2JGrZ4gRKwz14 1000
//!

pub(crate) mod cli;
mod commands;
mod community_spec;
mod utils;

use clap::Parser;
use cli::Cli;

use encointer_node_runtime::BalanceType;

const PREFUNDING_NR_OF_TRANSFER_EXTRINSICS: u128 = 1000;

mod exit_code {
	pub const WRONG_PHASE: i32 = 50;
	pub const FEE_PAYMENT_FAILED: i32 = 51;
	pub const INVALID_REPUTATION: i32 = 52;
	pub const RPC_ERROR: i32 = 60;
	pub const NOT_CC_HOLDER: i32 = 61;
	pub const NO_CID_SPECIFIED: i32 = 70;
}

#[tokio::main]
async fn main() {
	env_logger::init();
	let cli = Cli::parse();
	commands::run(&cli).await;
}
