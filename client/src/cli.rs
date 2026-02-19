use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
	name = "encointer-client-notee",
	version,
	author = "Encointer Association <info@encointer.org>",
	about = "interact with encointer-node-notee"
)]
pub struct Cli {
	#[arg(short = 'u', long, global = true, default_value = "ws://127.0.0.1")]
	pub node_url: String,

	#[arg(short = 'p', long, global = true, default_value = "9944")]
	pub node_port: String,

	#[arg(short = 'c', long, global = true, help = "community identifier, base58 encoded")]
	pub cid: Option<String>,

	#[arg(
		long = "tx-payment-cid",
		global = true,
		help = "cid of the community currency in which tx fees should be paid"
	)]
	pub tx_payment_cid: Option<String>,

	#[arg(long = "at", global = true, help = "block hash at which to query")]
	pub at: Option<String>,

	#[arg(short = 'v', long, global = true, help = "print extra information")]
	pub verbose: bool,

	#[command(subcommand)]
	pub command: Commands,
}

impl Cli {
	pub fn at_block(&self) -> Option<sp_core::H256> {
		self.at.as_ref().map(|hex| {
			let vec = sp_core::bytes::from_hex(hex)
				.unwrap_or_else(|_| panic!("bytes::from_hex failed, data is: {hex}"));
			assert!(vec.len() == 32, "block hash must be 32 bytes");
			sp_core::H256::from_slice(&vec)
		})
	}
}

#[derive(Subcommand)]
pub enum Commands {
	#[command(flatten)]
	Account(AccountCmd),
	#[command(flatten)]
	Chain(ChainCmd),
	/// Ceremony-related commands
	Ceremony {
		#[command(subcommand)]
		cmd: CeremonyCmd,
	},
	/// Community-related commands
	Community {
		#[command(subcommand)]
		cmd: CommunityCmd,
	},
	/// Bazaar-related commands
	Bazaar {
		#[command(subcommand)]
		cmd: BazaarCmd,
	},
	/// Faucet-related commands
	Faucet {
		#[command(subcommand)]
		cmd: FaucetCmd,
	},
	/// Reputation-related commands
	Reputation {
		#[command(subcommand)]
		cmd: ReputationCmd,
	},
	/// Democracy-related commands
	Democracy {
		#[command(subcommand)]
		cmd: DemocracyCmd,
	},
	/// Offline payment-related commands
	#[command(name = "offline-payment")]
	OfflinePayment {
		#[command(subcommand)]
		cmd: OfflinePaymentCmd,
	},
	/// IPFS-related commands
	Ipfs {
		#[command(subcommand)]
		cmd: IpfsCmd,
	},
}

// -- Account (flattened top-level) --

#[derive(Subcommand)]
pub enum AccountCmd {
	/// Import account into key store (creates new or uses supplied seed)
	NewAccount {
		/// Seed, mnemonic or SURI
		seed: Option<String>,
	},
	/// List all accounts in keystore
	ListAccounts,
	/// Print mnemonic phrase for a keystore account
	ExportSecret {
		/// AccountId in SS58 format
		account: String,
	},
	/// Send bootstrapping funds to account(s)
	Fund {
		/// Account(s) to fund, SS58 encoded
		#[arg(required = true, num_args = 1..)]
		fundees: Vec<String>,
	},
}

// -- Chain (flattened top-level) --

#[derive(Subcommand)]
pub enum ChainCmd {
	/// Query balance for AccountId (native if no --cid, community currency if --cid)
	Balance {
		/// AccountId in SS58 format
		account: String,
		/// List all community currency balances
		#[arg(short = 'a', long)]
		all: bool,
	},
	/// Query total issuance for community (requires --cid)
	Issuance,
	/// Transfer funds between accounts
	Transfer {
		/// Sender's AccountId (SS58)
		from: String,
		/// Recipient's AccountId (SS58)
		to: String,
		/// Amount to transfer
		amount: String,
		/// Print encoded call instead of sending
		#[arg(short = 'd', long)]
		dryrun: bool,
	},
	/// Transfer all community currency funds (requires --cid)
	#[command(name = "transfer-all")]
	TransferAll {
		/// Sender's AccountId (SS58)
		from: String,
		/// Recipient's AccountId (SS58)
		to: String,
	},
	/// Listen to on-chain events
	Listen {
		/// Exit after N encointer events
		#[arg(short = 'e', long = "await-events")]
		events: Option<u32>,
		/// Exit after N blocks
		#[arg(short = 'b', long = "await-blocks")]
		blocks: Option<u32>,
	},
	/// Query node metadata as JSON
	PrintMetadata,
}

// -- Ceremony --

#[derive(Subcommand)]
pub enum CeremonyCmd {
	/// Register for ceremony
	Register {
		/// AccountId (SS58)
		account: String,
		/// Account which signs the tx
		#[arg(short = 's', long)]
		signer: Option<String>,
	},
	/// Unregister from ceremony
	Unregister {
		/// AccountId (SS58)
		account: String,
		/// Account which signs the tx
		#[arg(short = 's', long)]
		signer: Option<String>,
		/// Ceremony index (negative = relative to current)
		#[arg(allow_hyphen_values = true)]
		ceremony_index: Option<i32>,
	},
	/// Upgrade registration to reputable
	UpgradeRegistration {
		/// AccountId (SS58)
		account: String,
		/// Account which signs the tx
		#[arg(short = 's', long)]
		signer: Option<String>,
	},
	/// Endorse newcomers with a bootstrapper account
	Endorse {
		/// Bootstrapper account (SS58)
		bootstrapper: String,
		/// Endorsee account(s) (SS58)
		#[arg(short = 'e', long = "endorsees", required = true, num_args = 1..)]
		endorsees: Vec<String>,
	},
	/// Claim attendance for ceremony
	Attest {
		/// AccountId (SS58)
		account: String,
		/// Attestee accounts (SS58, min 2)
		#[arg(required = true, num_args = 2..)]
		attestees: Vec<String>,
	},
	/// Create attendance claim
	NewClaim {
		/// AccountId (SS58)
		account: String,
		/// Vote on number of people present
		vote: u32,
	},
	/// Claim meetup rewards
	ClaimReward {
		/// Account which signs the tx
		#[arg(short = 's', long)]
		signer: Option<String>,
		/// Meetup index to claim for
		#[arg(long)]
		meetup_index: Option<u64>,
		/// Claim for all meetups
		#[arg(short = 'a', long)]
		all: bool,
	},
	/// List registered participants
	ListParticipants {
		/// Ceremony index (negative = relative to current)
		#[arg(allow_hyphen_values = true)]
		ceremony_index: Option<i32>,
	},
	/// List assigned meetups
	ListMeetups {
		/// Ceremony index (negative = relative to current)
		#[arg(allow_hyphen_values = true)]
		ceremony_index: Option<i32>,
	},
	/// List attestees
	ListAttestees {
		/// Ceremony index (negative = relative to current)
		#[arg(allow_hyphen_values = true)]
		ceremony_index: Option<i32>,
	},
	/// List reputables
	ListReputables,
	/// Print ceremony statistics as JSON
	Stats {
		/// Ceremony index (negative = relative to current)
		#[arg(long = "ceremony-index", allow_hyphen_values = true)]
		ceremony_index: Option<i32>,
	},
	/// Get proof of attendance
	GetProofOfAttendance {
		/// AccountId (SS58)
		account: String,
		/// Ceremony index (negative = relative to current)
		#[arg(long = "ceremony-index", allow_hyphen_values = true)]
		ceremony_index: Option<i32>,
	},
	/// List reputation history
	GetReputation {
		/// AccountId (SS58)
		account: String,
	},
	/// Purge ceremony history for a community
	Purge {
		/// First ceremony index to purge
		from_cindex: i32,
		/// Last ceremony index to purge
		to_cindex: i32,
	},
	/// Set meetup time offset
	SetMeetupTimeOffset {
		/// Signed offset in milliseconds
		#[arg(long = "time-offset", allow_hyphen_values = true)]
		time_offset: i32,
	},
	/// Read current ceremony phase
	GetPhase,
	/// Advance to next ceremony phase (ROOT)
	NextPhase {
		/// Account with privileges (sudo or councillor)
		#[arg(short = 's', long)]
		signer: Option<String>,
	},
	/// Read current ceremony index
	GetCindex,
	/// Get bootstrappers with remaining newbie tickets
	GetBootstrapperTickets,
}

// -- Community --

#[derive(Subcommand)]
pub enum CommunityCmd {
	/// Register new community
	New {
		/// Enhanced geojson file specifying the community
		specfile: String,
		/// Account with necessary privileges
		#[arg(short = 's', long)]
		signer: Option<String>,
		/// Print encoded call instead of sending
		#[arg(short = 'd', long)]
		dryrun: bool,
		/// Call wrapping: none|sudo|collective
		#[arg(short = 'w', long = "wrap-call", default_value = "none")]
		wrap_call: String,
		/// Maximum batch size
		#[arg(long = "batch-size", default_value = "100")]
		batch_size: u32,
	},
	/// List all registered communities
	List,
	/// List all meetup locations for a community
	ListLocations,
	/// Register new locations for a community
	AddLocations {
		/// Geojson file with locations as points
		specfile: String,
		/// Account with necessary privileges
		#[arg(short = 's', long)]
		signer: Option<String>,
		/// Print encoded call instead of sending
		#[arg(short = 'd', long)]
		dryrun: bool,
	},
	/// Get community treasury account
	GetTreasury,
	/// Query swap native option for an account
	GetSwapNativeOption {
		/// AccountId (SS58)
		account: String,
	},
	/// Query swap asset option for an account
	GetSwapAssetOption {
		/// AccountId (SS58)
		account: String,
	},
	/// Exercise a swap native option
	SwapNative {
		/// AccountId (SS58)
		account: String,
		/// Desired amount of native tokens to receive
		amount: u128,
	},
	/// Exercise a swap asset option
	SwapAsset {
		/// AccountId (SS58)
		account: String,
		/// Desired amount of asset tokens to receive
		amount: u128,
	},
	/// Remove a location for a community
	RemoveLocation {
		/// Account with necessary privileges
		#[arg(short = 's', long)]
		signer: Option<String>,
		/// Print encoded call instead of sending
		#[arg(short = 'd', long)]
		dryrun: bool,
		/// Geohash of the location
		#[arg(short = 'g', long)]
		geohash: Option<String>,
		/// Location index to remove
		#[arg(short = 'l', long)]
		location_index: Option<u32>,
	},
}

// -- Bazaar --

#[derive(Subcommand)]
pub enum BazaarCmd {
	/// Register a community business
	CreateBusiness {
		/// Business owner AccountId (SS58)
		account: String,
		/// IPFS content identifier
		#[arg(long = "ipfs-cid")]
		ipfs_cid: String,
	},
	/// Update an existing community business
	UpdateBusiness {
		/// Business owner AccountId (SS58)
		account: String,
		/// IPFS content identifier
		#[arg(long = "ipfs-cid")]
		ipfs_cid: String,
	},
	/// Create an offering for a business
	CreateOffering {
		/// Business owner AccountId (SS58)
		account: String,
		/// IPFS content identifier
		#[arg(long = "ipfs-cid")]
		ipfs_cid: String,
	},
	/// List businesses for a community
	ListBusinesses,
	/// List offerings for a community
	ListOfferings,
	/// List offerings for a specific business
	ListBusinessOfferings {
		/// Business owner AccountId (SS58)
		account: String,
	},
}

// -- Faucet --

#[derive(Subcommand)]
pub enum FaucetCmd {
	/// Create a faucet
	Create {
		/// Creator AccountId (SS58)
		account: String,
		/// Faucet name
		faucet_name: String,
		/// Faucet balance
		faucet_balance: u128,
		/// Drip amount
		faucet_drip_amount: u128,
		/// Whitelist of CIDs
		whitelist: Vec<String>,
	},
	/// Drip from a faucet
	Drip {
		/// AccountId (SS58)
		account: String,
		/// Faucet account (SS58)
		faucet_account: String,
		/// Ceremony index
		cindex: i32,
	},
	/// Dissolve a faucet (root only)
	Dissolve {
		/// Account with privileges
		#[arg(short = 's', long)]
		signer: Option<String>,
		/// Faucet account (SS58)
		faucet_account: String,
		/// Beneficiary of remaining funds (SS58)
		faucet_beneficiary: String,
	},
	/// Close an empty faucet
	Close {
		/// Creator AccountId (SS58)
		account: String,
		/// Faucet account (SS58)
		faucet_account: String,
	},
	/// Set faucet reserve amount (root)
	SetReserveAmount {
		/// Account with privileges
		#[arg(short = 's', long)]
		signer: Option<String>,
		/// Reserve amount
		faucet_reserve_amount: u128,
	},
	/// List all faucets
	List,
}

// -- Reputation --

#[derive(Subcommand)]
pub enum ReputationCmd {
	/// Register a Bandersnatch public key
	RegisterKey {
		/// AccountId (SS58)
		account: String,
		/// Hex-encoded 32-byte Bandersnatch key (auto-derived if omitted)
		#[arg(long)]
		key: Option<String>,
	},
	/// Initiate ring computation
	InitiateRings {
		/// AccountId (SS58)
		account: String,
		/// Ceremony index
		#[arg(long = "ceremony-index")]
		ceremony_index: u32,
	},
	/// Continue pending ring computation
	ContinueRings {
		/// AccountId (SS58)
		account: String,
	},
	/// Query ring members
	GetRings {
		/// Ceremony index
		#[arg(long = "ceremony-index")]
		ceremony_index: u32,
	},
	/// Produce ring-VRF proof of personhood
	ProvePersonhood {
		/// AccountId (SS58)
		account: String,
		/// Ceremony index
		#[arg(long = "ceremony-index")]
		ceremony_index: u32,
		/// Attendance level (1-5)
		#[arg(long, default_value = "1")]
		level: u8,
		/// Sub-ring index
		#[arg(long = "sub-ring", default_value = "0")]
		sub_ring: u32,
	},
	/// Verify ring-VRF proof of personhood
	VerifyPersonhood {
		/// Hex-encoded ring-VRF signature
		#[arg(long)]
		signature: String,
		/// Ceremony index
		#[arg(long = "ceremony-index")]
		ceremony_index: u32,
		/// Attendance level (1-5)
		#[arg(long, default_value = "1")]
		level: u8,
		/// Sub-ring index
		#[arg(long = "sub-ring", default_value = "0")]
		sub_ring: u32,
	},
	/// List reputation commitments
	ListCommitments {
		/// Filter by purpose ID
		#[arg(long = "purpose-id")]
		purpose_id: Option<u64>,
	},
	/// List reputation commitment purposes
	ListPurposes,
}

// -- Democracy --

#[derive(Subcommand)]
pub enum DemocracyCmd {
	/// Submit set inactivity timeout proposal
	SubmitSetInactivityTimeout {
		/// AccountId (SS58)
		account: String,
		/// Inactivity timeout value
		inactivity_timeout: u32,
	},
	/// Submit update nominal income proposal
	SubmitUpdateNominalIncome {
		/// AccountId (SS58)
		account: String,
		/// New nominal income
		nominal_income: f64,
	},
	/// Submit update demurrage proposal
	SubmitUpdateDemurrage {
		/// AccountId (SS58)
		account: String,
		/// Demurrage halving blocks
		demurrage_halving_blocks: u64,
	},
	/// Submit a petition
	SubmitPetition {
		/// AccountId (SS58)
		account: String,
		/// What the petition demands
		demand: String,
	},
	/// Submit spend native proposal
	SubmitSpendNative {
		/// AccountId (SS58)
		account: String,
		/// Beneficiary (SS58)
		to: String,
		/// Amount
		amount: u128,
	},
	/// Submit proposal to issue a swap native option
	SubmitIssueSwapNativeOption {
		/// AccountId (SS58)
		account: String,
		/// Beneficiary (SS58)
		to: String,
		/// Total native token allowance
		#[arg(long = "native-allowance")]
		native_allowance: u128,
		/// CC per native token exchange rate (omit for oracle/auction)
		#[arg(long)]
		rate: Option<f64>,
		/// Burn CC instead of sending to treasury
		#[arg(long = "do-burn")]
		do_burn: bool,
		/// First time of validity (unix timestamp in milliseconds)
		#[arg(long = "valid-from")]
		valid_from: Option<u64>,
		/// Expiry time (unix timestamp in milliseconds)
		#[arg(long = "valid-until")]
		valid_until: Option<u64>,
	},
	/// Submit proposal to issue a swap asset option
	SubmitIssueSwapAssetOption {
		/// AccountId (SS58)
		account: String,
		/// Beneficiary (SS58)
		to: String,
		/// SCALE-encoded VersionedLocatableAsset (hex)
		#[arg(long = "asset-id")]
		asset_id: String,
		/// Total asset token allowance
		#[arg(long = "asset-allowance")]
		asset_allowance: u128,
		/// CC per asset token exchange rate (omit for oracle/auction)
		#[arg(long)]
		rate: Option<f64>,
		/// Burn CC instead of sending to treasury
		#[arg(long = "do-burn")]
		do_burn: bool,
		/// First time of validity (unix timestamp in milliseconds)
		#[arg(long = "valid-from")]
		valid_from: Option<u64>,
		/// Expiry time (unix timestamp in milliseconds)
		#[arg(long = "valid-until")]
		valid_until: Option<u64>,
	},
	/// List proposals
	ListProposals {
		/// Include failed proposals
		#[arg(short = 'a', long)]
		all: bool,
	},
	/// List enactment queue
	ListEnactments,
	/// Submit a vote
	Vote {
		/// AccountId (SS58)
		account: String,
		/// Proposal ID
		proposal_id: u128,
		/// Vote: aye or nay
		vote: String,
		/// Reputation: cid1_cindex1,cid2_cindex2,...
		reputation_vec: String,
	},
	/// Update proposal state
	UpdateProposalState {
		/// AccountId (SS58)
		account: String,
		/// Proposal ID
		proposal_id: u128,
	},
}

// -- Offline Payment --

#[derive(Subcommand)]
pub enum OfflinePaymentCmd {
	/// Register offline payment identity (ZK commitment)
	RegisterIdentity {
		/// AccountId (SS58)
		account: String,
	},
	/// Get offline identity commitment
	GetIdentity {
		/// AccountId (SS58)
		account: String,
	},
	/// Generate offline payment proof (JSON)
	Generate {
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
	Submit {
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
	/// Generate trusted setup (PK + VK)
	GenerateTrustedSetup {
		/// Output path for proving key
		#[arg(long = "pk-out", default_value = "proving_key.bin")]
		pk_out: String,
		/// Output path for verifying key
		#[arg(long = "vk-out", default_value = "verifying_key.bin")]
		vk_out: String,
	},
	/// Verify trusted setup consistency
	VerifyTrustedSetup {
		/// Path to proving key
		#[arg(long)]
		pk: String,
		/// Path to verifying key
		#[arg(long)]
		vk: String,
	},
	/// Initialize multiparty trusted setup ceremony
	CeremonyInit {
		/// Output path for ceremony PK
		#[arg(long = "pk-out", default_value = "ceremony_pk.bin")]
		pk_out: String,
		/// Output path for transcript
		#[arg(long, default_value = "ceremony_transcript.json")]
		transcript: String,
	},
	/// Apply ceremony contribution
	CeremonyContribute {
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
	CeremonyVerify {
		/// Path to ceremony PK
		#[arg(long, default_value = "ceremony_pk.bin")]
		pk: String,
		/// Path to transcript
		#[arg(long, default_value = "ceremony_transcript.json")]
		transcript: String,
	},
	/// Finalize ceremony — extract PK and VK
	CeremonyFinalize {
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
	/// Inspect a key file
	InspectKey {
		/// Path to key file
		#[arg(long)]
		file: String,
	},
}

// -- IPFS --

#[derive(Subcommand)]
pub enum IpfsCmd {
	/// Upload file to IPFS via authenticated gateway
	Upload {
		/// Account to authenticate (must be CC holder)
		#[arg(short = 's', long)]
		signer: String,
		/// IPFS auth gateway URL
		#[arg(long, default_value = "http://localhost:5050")]
		gateway: String,
		/// Path to file to upload
		file_path: String,
	},
}
