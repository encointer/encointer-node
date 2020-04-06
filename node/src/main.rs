//! Substrate Node Template CLI library.
#![warn(missing_docs)]

mod chain_spec;
#[macro_use]
mod service;
mod cli;
mod command;

fn main() -> sc_cli::Result<()> {
	let version = sc_cli::VersionInfo {
		name: "Encointer Node",
		commit: env!("VERGEN_SHA_SHORT"),
		version: env!("CARGO_PKG_VERSION"),
		executable_name: "encointer-node",
		author: "Alain Brenzikofer",
		description: "Encointer Node",
		support_url: "alain@encointer.org",
		copyright_start_year: 2019,
	};

	command::run(version)
}
