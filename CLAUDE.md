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
