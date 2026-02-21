# Encointer Node

Substrate-based blockchain node for the Encointer protocol.

## Structure

```
node/     - Node binary (networking, consensus, RPC)
runtime/  - WASM runtime (pallets, weights, genesis)
cli/      - CLI client and API client library
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
cargo build --release -p encointer-node-runtime
```

## formatting and linting

```
# Format Rust code (requires nightly)
cargo +nightly fmt

# Format TOML files
taplo fmt
```

## WASM Runtime Size

The runtime WASM is at `target/release/wbuild/encointer-node-runtime/`:
- `encointer_node_runtime.compact.compressed.wasm` - production artifact

Compare sizes:
```bash
ls -la target/release/wbuild/encointer-node-runtime/*.wasm
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

## Bot Community Simulation (`cli/`)

### Running locally

Must run from `cli/` directory (keystore, typedefs.json, community specs are relative paths).

```bash
cd client

# 1. Start node
../target/release/encointer-node --dev --tmp --rpc-port 9944 \
  --enable-offchain-indexing true --rpc-methods unsafe &

# 2. Start ceremony phase and faucet service
PYTHONUNBUFFERED=1 python3 ceremony-phase-and-faucet-service.py &

# 3. Init community (purge keystore first)
rm -rf my_keystore
python3 bot-community.py init

# 5. Simulate
python3 bot-community.py simulate --ceremonies 7
```

### Architecture

- `ceremony-phase-and-faucet-service.py` — Flask HTTP service on port 7070. Combines phase coordination (barrier-based: communities register and signal readiness via HTTP) and faucet (funds accounts via `//Alice`). All `//Alice` operations serialized via lock, eliminating nonce clashes.
- `bot-community.py` — orchestrator: init creates community, simulate runs N ceremonies. Coordinates with the service via `/register`, `/ready`, `/unregister` HTTP calls.
- `py_client/agent_pool.py` — core agent logic: registration, attestation, growth, auxiliary features, assertions.
- `py_client/campaign_*.py` — modular campaign plugins (personhood, offline payment, swap option).

### Key patterns

- **Parallelization**: `ThreadPoolExecutor(max_workers=100)` for independent CLI calls (registration, attestation, voting, key registration). Safe because each account has its own nonce.
- **Early key registration**: Bandersnatch keys and offline identities are registered at account creation (bootstrap/grow), not during Registering phase. This populates rings from ceremony 1.
- **Stats capture timing**: `_write_current_stats()` runs before `grow()` in execute_registering (total_supply gates growth). So stats for ceremony N reflect pre-growth state; growth shows up in ceremony N+1's stats.

### Gotchas

- `PYTHONUNBUFFERED=1` required for real-time log output (Python buffers stdout when piped).
- Keystore prompt: delete `my_keystore/` before `init` to avoid interactive y/n prompt.
- Assertion timing: population > 10 only valid from cindex >= 3 (growth happens during ceremony 2's Registering but stats captured before growth).
