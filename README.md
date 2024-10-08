# encointer-node

Encointer-node is the implementation of the [encointer.org](https://encointer.org) blockchain.
Use this together with the mobile phone
app [encointer mobile app](https://github.com/encointer/encointer-wallet-flutter)

The cli client is based on [substrate-api-client](https://github.com/scs/substrate-api-client)
The Trusted Execution version for Testnet Cantillon is on
branch [sgx-master](https://github.com/encointer/encointer-node/tree/sgx-master) based
on [substraTEE project](https://github.com/scs/substraTEE).

## Building

Install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh
```

Install required tools:

```bash
// with a rust-toolchain.toml rustup automatically installs the correct tools.
rustup show
```

Build the node:

```bash
cargo build --release
```

## Run Dev Node

You can start a development chain with:

```bash
export RUST_LOG=INFO,parity_ws=WARN,sc_basic_authorship=warn,aura=warn,encointer=debug
./target/release/encointer-node-notee --dev --enable-offchain-indexing true
```

Offchain-indexing is needed for the custom rpc `encointer_getAllCommunities`. If you don't want it, omit the flag.
`--rpc-methods unsafe` is needed for the bazaar's business and offering aggregation rpcs.

Additional CLI usage options are available and may be shown by running `./target/release/encointer-node-notee --help`.

### Test state migrations

Compile the node with:

```bash
cargo build --release --features try-runtime
```

Then test state migrations with against live data on the Gesell network with:

```bash
try-runtime --runtime target/release/wbuild/encointer-node-notee-runtime/encointer_node_notee_runtime.compact.compressed.wasm on-runtime-upgrade --checks=pre-and-post --disable-spec-version-check live --uri wss://gesell.encointer.org:443
```

### Run with docker

```bash
# Expose the node's ports to the host with the -p flag.
docker run -p 30333:30333 -p 9944:9944 -p 9933:9933 -p 9615:9615 \
  encointer/encointer-node-notee:0.0.2 \
  --dev \
  --enable-offchain-indexing true \
  --rpc-methods unsafe \
  -lencointer=debug,parity_ws=warn \
  --ws-external \
  --rpc-external
```

## Run Testnet Gesell Node

Join our testnet as a full node with

```bash
RUST_LOG=INFO,parity_ws=WARN,sc_basic_authorship=warn,aura=warn,encointer=debug
./target/release/encointer-node-notee --chain gesellv4SpecRaw.json --enable-offchain-indexing true --rpc-cors all
```

## CLI client

We currently have limited support for the [polkadot-js apps](https://polkadot.js.org/apps) UI. Encointer comes with a
cli application instead that supports all interactions with the chain

### Run Client

```bash
encointer-node/client> cargo build --release
encointer-node/client> ../target/release/encointer-client-notee transfer //Alice 5GziKpBELV7fuYNy7quQfWGgVARn8onchS86azuPQkFj9nEZ 1000000
encointer-node/client> ../target/release/encointer-client-notee list_participant_registry
encointer-node/client> ../target/release/encointer-client-notee list_meetup_registry
encointer-node/client> ../target/release/encointer-client-notee list_witnesses_registry
encointer-node/client> ../target/release/encointer-client-notee --help
```

The master of ceremony can play fast-forward for demo purposes (ceremonies only happen ~monthly. not good for demos)

```bash
encointer-node/client> ./encointer-client-notee next_phase
```

To run a full demo (you may need to fix ports in the scripts if you change them):

```
encointer-node/client> ./bootstrap_demo_community.sh
```

### Run with docker

```bash
docker run -it encointer/encointer-client-notee:<version> [encointer-client-notee|bootstrap_demo_community.py|cli.py] <params>

# Example to talk to a node on the host.
docker run -it encointer/encointer-client-notee:<version> encointer-client-notee list-communities -u ws://host.docker.internal -p 9944

# Bootstrap demo community on a node on the host machine.
docker run -it encointer-client-notee:dev bootstrap_demo_community.py -u ws://host.docker.internal -p 9944
```

### Grow Bot Community

Assuming a local node is running with default ports:

```bash
pip3 install pip install geojson pyproj RandomWords substrate-interface
# in first terminal, do this to accelerate phase progress
./phase.py --idle-blocks 3
# in second terminal, launch faucet service
./faucet.py
# in third terminal, populate your bot community
./bot-community.py init
./bot-community.py benchmark
```

## Web UI

There is no fully featured UI yet, but you can use [polkadot-js apps](https://github.com/polkadot-js/apps).
This allows you to explore chain state but it doesn't support all types of extrinsic parameters needed. Use our CLI
client instead.

## Mobile App

The PoC1 Android App doesn't work with this release anymore, but you can watch progress
at [encointer-app](https://github.com/encointer/encointer-app)

## Dev-Remarks

### Benchmarking

For benchmarking a new pallet you need to do the following:

1. Add the new pallet to be benchmarked to the `define_benchmarks!` macro in the runtime.
2. Make sure you enable the pallet's benchmark by enabling its runtime-benchmark feature in the runtime's toml.
3. Compile the node with `--features runtime-benchmarks`
4. Add it to the benchmark script: `./scripts/benchmark_runtime.sh`

This will automatically generate the new/updated weight file in `./runtime/src/weights`.
