use clap::{App, Arg, ArgMatches};

const ACCOUNT_ARG: &'static str = "accountid";
const SIGNER_ARG: &'static str = "signer";
const CID_ARG: &'static str = "cid";
const CLAIMS_ARG: &'static str = "claims";
const CEREMONY_INDEX_ARG: &'static str = "ceremony-index";
const IPFS_CID_ARG: &'static str = "ceremony-index";
const BOOTSTRAPPER_ARG: &'static str = "bootstrapper";
const FUNDEES_ARG: &'static str = "fundees";
const ENDORSEES_ARG: &'static str = "endorsees";
const ALL_FLAG: &'static str = "all";

pub trait EncointerArgs<'b> {
	fn account_arg(self) -> Self;
	fn signer_arg(self, help: &'b str) -> Self;
	fn optional_cid_arg(self) -> Self;
	fn claims_arg(self) -> Self;
	fn ceremony_index_arg(self) -> Self;
	fn ipfs_cid_arg(self) -> Self;
	fn bootstrapper_arg(self) -> Self;
	fn fundees_arg(self) -> Self;
	fn endorsees_arg(self) -> Self;
	fn all_flag(self) -> Self;
}

pub trait EncointerArgsExtractor {
	fn account_arg(&self) -> Option<&str>;
	fn signer_arg(&self) -> Option<&str>;
	fn cid_arg(&self) -> Option<&str>;
	fn claims_arg(&self) -> Option<Vec<&str>>;
	fn ceremony_index_arg(&self) -> Option<&str>;
	fn ipfs_cid_arg(&self) -> Option<&str>;
	fn bootstrapper_arg(&self) -> Option<&str>;
	fn fundees_arg(&self) -> Option<Vec<&str>>;
	fn endorsees_arg(&self) -> Option<Vec<&str>>;
	fn all_flag(&self) -> bool;
}

impl<'a, 'b> EncointerArgs<'b> for App<'a, 'b> {
	fn account_arg(self) -> Self {
		self.arg(
			Arg::with_name(ACCOUNT_ARG)
				.takes_value(true)
				.required(true)
				.value_name("SS58")
				.help("AccountId in ss58check format"),
		)
	}

	fn signer_arg(self, help: &'b str) -> Self {
		self.arg(
			Arg::with_name(SIGNER_ARG)
				.takes_value(true)
				.required(true)
				.value_name("SS58")
				.help(help),
		)
	}

	fn optional_cid_arg(self) -> Self {
		self.arg(
			Arg::with_name(CID_ARG)
				.short("c")
				.long("cid")
				.global(true)
				.takes_value(true)
				.value_name("STRING")
				.help("community identifier, base58 encoded"),
		)
	}

	fn claims_arg(self) -> Self {
		self.arg(
			Arg::with_name(CLAIMS_ARG)
				.takes_value(true)
				.required(true)
				.multiple(true)
				.min_values(2),
		)
	}

	fn ceremony_index_arg(self) -> Self {
		self.arg(
			Arg::with_name(CEREMONY_INDEX_ARG)
				.takes_value(true)
				.allow_hyphen_values(true)
				.default_value("-1")
				.help(
					"If positive, absolute index. If negative, current_index -i. 0 is not allowed",
				),
		)
	}

	fn ipfs_cid_arg(self) -> Self {
		self.arg(
			Arg::with_name(IPFS_CID_ARG)
				.long("ipfs-cid")
				.required(true)
				.takes_value(true)
				.value_name("STRING")
				.help("ipfs content identifier, base58 encoded"),
		)
	}

	fn bootstrapper_arg(self) -> Self {
		self.arg(
			Arg::with_name(BOOTSTRAPPER_ARG)
				.takes_value(true)
				.required(true)
				.value_name("SS58")
				.help("Bootstrapper in ss58check format"),
		)
	}

	fn fundees_arg(self) -> Self {
		self.arg(
			Arg::with_name(FUNDEES_ARG)
				.takes_value(true)
				.required(true)
				.value_name("FUNDEE")
				.multiple(true)
				.min_values(1)
				.help("Account(s) to be funded, ss58check encoded"),
		)
	}

	fn endorsees_arg(self) -> Self {
		self.arg(
			Arg::with_name(ENDORSEES_ARG)
				.short("-e")
				.long("-endorsees")
				.takes_value(true)
				.required(true)
				.value_name("ENDORSEE")
				.multiple(true)
				.min_values(1)
				.help("Account(s) to be endorsed, ss58check encoded"),
		)
	}

	fn all_flag(self) -> Self {
		self.arg(
			Arg::with_name(ALL_FLAG)
				.short("a")
				.long("all")
				.takes_value(false)
				.required(false)
				.help("list all community currency balances for account"),
		)
	}
}

impl<'a> EncointerArgsExtractor for ArgMatches<'a> {
	fn account_arg(&self) -> Option<&str> {
		self.value_of(ACCOUNT_ARG)
	}

	fn signer_arg(&self) -> Option<&str> {
		self.value_of(SIGNER_ARG)
	}

	fn cid_arg(&self) -> Option<&str> {
		self.value_of(CID_ARG)
	}

	fn claims_arg(&self) -> Option<Vec<&str>> {
		self.values_of(CLAIMS_ARG).map(|c| c.collect())
	}

	fn ceremony_index_arg(&self) -> Option<&str> {
		self.value_of(CEREMONY_INDEX_ARG)
	}

	fn ipfs_cid_arg(&self) -> Option<&str> {
		self.value_of(IPFS_CID_ARG)
	}

	fn bootstrapper_arg(&self) -> Option<&str> {
		self.value_of(BOOTSTRAPPER_ARG)
	}

	fn fundees_arg(&self) -> Option<Vec<&str>> {
		self.values_of(FUNDEES_ARG).map(|v| v.collect())
	}

	fn endorsees_arg(&self) -> Option<Vec<&str>> {
		self.values_of(ENDORSEES_ARG).map(|v| v.collect())
	}

	fn all_flag(&self) -> bool {
		self.is_present(ALL_FLAG)
	}
}
