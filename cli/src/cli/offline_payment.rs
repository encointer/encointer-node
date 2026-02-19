use clap::Subcommand;

use super::Cli;

#[derive(Subcommand)]
pub enum OfflinePaymentCmd {
	/// Generate offline payment proof (JSON)
	Pay {
		/// Sender account
		#[arg(short = 's', long)]
		signer: Option<String>,
		/// Recipient (SS58)
		#[arg(long)]
		to: String,
		/// Amount
		#[arg(long)]
		amount: String,
		/// Path to proving key file
		#[arg(long = "pk-file")]
		pk_file: Option<String>,
	},
	/// Submit offline payment proof
	Settle {
		/// Account to sign the transaction
		#[arg(short = 's', long)]
		signer: Option<String>,
		/// Path to JSON proof file
		#[arg(long = "proof-file")]
		proof_file: Option<String>,
		/// Hex-encoded proof
		#[arg(long)]
		proof: Option<String>,
		/// Sender AccountId (SS58)
		#[arg(long)]
		sender: Option<String>,
		/// Recipient AccountId (SS58)
		#[arg(long)]
		recipient: Option<String>,
		/// Transfer amount
		#[arg(long)]
		amount: Option<String>,
		/// Hex-encoded nullifier
		#[arg(long)]
		nullifier: Option<String>,
	},
	/// Admin commands
	#[command(subcommand)]
	Admin(OfflinePaymentAdminCmd),
}

#[derive(Subcommand)]
pub enum OfflinePaymentAdminCmd {
	/// Set Groth16 verification key (sudo)
	SetVk {
		/// Sudo account
		#[arg(short = 's', long)]
		signer: Option<String>,
		/// Path to verifying key file
		#[arg(long = "vk-file")]
		vk_file: Option<String>,
		/// Hex-encoded verification key
		#[arg(long)]
		vk: Option<String>,
	},
	/// Generate test verification key
	GenerateTestVk,
	/// Trusted setup commands
	#[command(subcommand)]
	TrustedSetup(TrustedSetupCmd),
	/// Setup ceremony commands
	#[command(subcommand)]
	Ceremony(SetupCeremonyCmd),
	/// Inspect a key file
	InspectKey {
		/// Path to key file
		#[arg(long)]
		file: String,
	},
}

#[derive(Subcommand)]
pub enum TrustedSetupCmd {
	/// Generate trusted setup (PK + VK)
	Generate {
		/// Output path for proving key
		#[arg(long = "pk-out", default_value = "proving_key.bin")]
		pk_out: String,
		/// Output path for verifying key
		#[arg(long = "vk-out", default_value = "verifying_key.bin")]
		vk_out: String,
	},
	/// Verify trusted setup consistency
	Verify {
		/// Path to proving key
		#[arg(long)]
		pk: String,
		/// Path to verifying key
		#[arg(long)]
		vk: String,
	},
}

#[derive(Subcommand)]
pub enum SetupCeremonyCmd {
	/// Initialize multiparty trusted setup ceremony
	Init {
		/// Output path for ceremony PK
		#[arg(long = "pk-out", default_value = "ceremony_pk.bin")]
		pk_out: String,
		/// Output path for transcript
		#[arg(long, default_value = "ceremony_transcript.json")]
		transcript: String,
	},
	/// Apply ceremony contribution
	Contribute {
		/// Path to ceremony PK
		#[arg(long, default_value = "ceremony_pk.bin")]
		pk: String,
		/// Path to transcript
		#[arg(long, default_value = "ceremony_transcript.json")]
		transcript: String,
		/// Participant name
		#[arg(long)]
		participant: String,
	},
	/// Verify ceremony contributions
	Verify {
		/// Path to ceremony PK
		#[arg(long, default_value = "ceremony_pk.bin")]
		pk: String,
		/// Path to transcript
		#[arg(long, default_value = "ceremony_transcript.json")]
		transcript: String,
	},
	/// Finalize ceremony — extract PK and VK
	Finalize {
		/// Path to ceremony PK (input)
		#[arg(long, default_value = "ceremony_pk.bin")]
		pk: String,
		/// Output path for final PK
		#[arg(long = "pk-out", default_value = "proving_key.bin")]
		pk_out: String,
		/// Output path for VK
		#[arg(long = "vk-out", default_value = "verifying_key.bin")]
		vk_out: String,
	},
}

impl OfflinePaymentCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::encointer_offline_payment;
		match self {
			Self::Pay { signer, to, amount, pk_file } =>
				encointer_offline_payment::generate_offline_payment(
					cli,
					signer.as_deref(),
					to,
					amount,
					pk_file.as_deref(),
				)
				.await,
			Self::Settle { signer, proof_file, proof, sender, recipient, amount, nullifier } =>
				encointer_offline_payment::submit_offline_payment(
					cli,
					signer.as_deref(),
					proof_file.as_deref(),
					proof.as_deref(),
					sender.as_deref(),
					recipient.as_deref(),
					amount.as_deref(),
					nullifier.as_deref(),
				)
				.await,
			Self::Admin(cmd) => cmd.run(cli).await,
		}
	}
}

impl OfflinePaymentAdminCmd {
	pub async fn run(&self, cli: &Cli) {
		use crate::commands::encointer_offline_payment;
		match self {
			Self::SetVk { signer, vk_file, vk } =>
				encointer_offline_payment::set_verification_key(
					cli,
					signer.as_deref(),
					vk_file.as_deref(),
					vk.as_deref(),
				)
				.await,
			Self::GenerateTestVk => encointer_offline_payment::generate_test_vk(),
			Self::TrustedSetup(cmd) => cmd.run(cli).await,
			Self::Ceremony(cmd) => cmd.run(cli).await,
			Self::InspectKey { file } => encointer_offline_payment::inspect_setup_key(file),
		}
	}
}

impl TrustedSetupCmd {
	pub async fn run(&self, _cli: &Cli) {
		use crate::commands::encointer_offline_payment;
		match self {
			Self::Generate { pk_out, vk_out } =>
				encointer_offline_payment::generate_trusted_setup(pk_out, vk_out),
			Self::Verify { pk, vk } => encointer_offline_payment::verify_trusted_setup(pk, vk),
		}
	}
}

impl SetupCeremonyCmd {
	pub async fn run(&self, _cli: &Cli) {
		use crate::commands::encointer_offline_payment;
		match self {
			Self::Init { pk_out, transcript } =>
				encointer_offline_payment::cmd_ceremony_init(pk_out, transcript),
			Self::Contribute { pk, transcript, participant } =>
				encointer_offline_payment::cmd_ceremony_contribute(pk, transcript, participant),
			Self::Verify { pk, transcript } =>
				encointer_offline_payment::cmd_ceremony_verify(pk, transcript),
			Self::Finalize { pk, pk_out, vk_out } =>
				encointer_offline_payment::cmd_ceremony_finalize(pk, pk_out, vk_out),
		}
	}
}
