use clap::{App, Arg, ArgMatches};

const ACCOUNT_ARG: &'static str = "accountid";
const SIGNER_ARG: &'static str = "signer";
const CLAIMS_ARG: &'static str = "claims";
const CEREMONY_INDEX_ARG: &'static str = "ceremony-index";

pub trait EncointerArgs<'b> {
	fn account_arg(self) -> Self;
	fn signer_arg(self, help: &'b str) -> Self;
	fn claims_arg(self) -> Self;
	fn ceremony_index_arg(self) -> Self;
}

pub trait EncointerArgsExtractor {
	fn account_arg(&self) -> Option<&str>;
	fn signer_arg(&self) -> Option<&str>;
	fn claims_arg(&self) -> Option<Vec<&str>>;
	fn ceremony_index_arg(&self) -> Option<&str>;
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
}

impl<'a> EncointerArgsExtractor for ArgMatches<'a> {
	fn account_arg(&self) -> Option<&str> {
		self.value_of(ACCOUNT_ARG)
	}

	fn signer_arg(&self) -> Option<&str> {
		self.value_of(SIGNER_ARG)
	}

	fn claims_arg(&self) -> Option<Vec<&str>> {
		self.values_of(CLAIMS_ARG).map(|c| c.collect())
	}

	fn ceremony_index_arg(&self) -> Option<&str> {
		self.value_of(CEREMONY_INDEX_ARG)
	}
}
