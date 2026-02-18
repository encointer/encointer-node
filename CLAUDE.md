# Encointer Node

Substrate-based blockchain node for the Encointer protocol.

## Structure

```
node/     - Node binary (networking, consensus, RPC)
runtime/  - WASM runtime (pallets, weights, genesis)
client/   - API client library
```

## Build & Run

```bash
# Check or clippy the entire workspace
# This skips the time-intensive building of the WASM runtimes
SKIP_WASM_BUILD=1 cargo check --workspace --all-targets --all-features
SKIP_WASM_BUILD=1 cargo clippy --workspace --all-targets --all-features

# Build release (includes WASM runtime)
cargo build --release

# Run dev node
./target/release/encointer-node --dev

# Build only the runtime WASM
cargo build --release -p encointer-node-notee-runtime
```

## formatting and linting

```
# Format Rust code (requires nightly)
cargo +nightly fmt

# Format TOML files
taplo fmt
```

## WASM Runtime Size

The runtime WASM is at `target/release/wbuild/encointer-node-notee-runtime/`:
- `encointer_node_notee_runtime.compact.compressed.wasm` - production artifact

Compare sizes:
```bash
ls -la target/release/wbuild/encointer-node-notee-runtime/*.wasm
```

The offline-payment pallet adds ~175 KB compressed due to arkworks BN254 pairing code.

## Pallet Dependencies

Pallets come from `encointer/pallets` repo. Two ways to configure:

### 1. Use git branch (default for CI)
In `Cargo.toml` `[patch.crates-io]`:
```toml
pallet-encointer-balances = { git = "https://github.com/encointer/pallets", branch = "master" }
```

### 2. Use local path (for development)
Uncomment the local path patches, comment out git patches:
```toml
pallet-encointer-balances = { path = "../encointer-pallets/balances" }
```

After switching, run `cargo update` to refresh the lockfile.

## Testing

```bash
# Unit tests
cargo test

# Integration tests require zombienet
# See zombienet/ directory for network configs
```

# CI

always consider github actions and attempt to make them work. If you test e2e locally, use similar tests

## Runtime Configuration

Key files:
- `runtime/src/lib.rs` - `construct_runtime!` macro, pallet configs
- `runtime/src/weights/` - Benchmark-generated weights
- `node/src/chain_spec.rs` - Genesis configuration

## Common Tasks

### Add a new pallet
1. Add dependency to `runtime/Cargo.toml` (with `default-features = false`)
2. Add to `construct_runtime!` in `runtime/src/lib.rs`
3. Implement `Config` trait for the runtime
4. Add git patch to root `Cargo.toml` `[patch.crates-io]` section

### Update pallet branch
Change branch in all `[patch.crates-io]` entries (there are ~22 of them).
Use find-replace: `branch = "old-branch"` → `branch = "new-branch"`

## Bot Community Simulation (`client/`)

### Running locally

Must run from `client/` directory (keystore, typedefs.json, community specs are relative paths).

```bash
cd client

# 1. Start node
../target/release/encointer-node-notee --dev --tmp --rpc-port 9944 \
  --enable-offchain-indexing true --rpc-methods unsafe &

# 2. Start phase controller (idle-blocks=3 for fast CI, 10 for relaxed)
PYTHONUNBUFFERED=1 python3 phase.py --idle-blocks 3 &

# 3. Start faucet HTTP service
PYTHONUNBUFFERED=1 python3 faucet.py &

# 4. Init community (purge keystore first)
rm -rf my_keystore
python3 bot-community.py init

# 5. Simulate
python3 bot-community.py simulate --ceremonies 7
```

### Architecture

- `phase.py` — watches block events, advances ceremony phase after N idle blocks. Has an "armed" gate: ignores idle blocks until the first user extrinsic is seen (prevents premature advancement on startup).
- `faucet.py` — Flask HTTP service on port 5000, funds accounts with native tokens via `//Alice`.
- `bot-community.py` — orchestrator: init creates community, simulate runs N ceremonies.
- `py_client/agent_pool.py` — core agent logic: registration, attestation, growth, auxiliary features, assertions.
- `py_client/campaign_*.py` — modular campaign plugins (personhood, offline payment, swap option).

### Key patterns

- **Parallelization**: `ThreadPoolExecutor(max_workers=100)` for independent CLI calls (registration, attestation, voting, key registration). Safe because each account has its own nonce.
- **Heartbeat**: `AgentPool.start_heartbeat()` sends periodic native transfers to prevent phase.py idle detection during read-heavy work (balance queries, ring queries, assertions). Uses native transfers (`cid=None`) so it works even before agents have CC balance.
- **Early key registration**: Bandersnatch keys and offline identities are registered at account creation (bootstrap/grow), not during Registering phase. This populates rings from ceremony 1.
- **Stats capture timing**: `_write_current_stats()` runs before `grow()` in execute_registering (total_supply gates growth). So stats for ceremony N reflect pre-growth state; growth shows up in ceremony N+1's stats.

### Gotchas

- `PYTHONUNBUFFERED=1` required for real-time log output (Python buffers stdout when piped).
- Keystore prompt: delete `my_keystore/` before `init` to avoid interactive y/n prompt.
- `pip install substrate-interface` (not `substrateinterface`) for phase.py dependency.
- Assertion timing: population > 10 only valid from cindex >= 3 (growth happens during ceremony 2's Registering but stats captured before growth).
