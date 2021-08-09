use clap::{App, Arg, ArgMatches};

const ACCOUNT_ARG: &'static str = "accountid";
const SIGNER_ARG: &'static str = "signer";

pub trait EncointerArgs<'b> {
	fn account_arg(self) -> Self;
	fn signer_arg(self, help: &'b str) -> Self;
}

pub trait EncointerArgsExtractor {
	fn account_arg(&self) -> Option<&str>;
	fn signer_arg(&self) -> Option<&str>;
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
}

impl<'a> EncointerArgsExtractor for ArgMatches<'a> {
	fn account_arg(&self) -> Option<&str> {
		self.value_of(ACCOUNT_ARG)
	}

	fn signer_arg(&self) -> Option<&str> {
		self.value_of(SIGNER_ARG)
	}
}
