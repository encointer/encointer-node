# encointer-node

![badge](https://img.shields.io/badge/substrate-2.0.0-success)

Encointer-node is the implementation of the [encointer.org](https://encointer.org) blockchain.
Use this together with the mobile phone app [encointer mobile app](https://github.com/encointer/encointer-wallet-flutter) 

The cli client is based on [substrate-api-client](https://github.com/scs/substrate-api-client)
The Trusted Execution version for Testnet Cantillon is on branch [sgx-master](https://github.com/encointer/encointer-node/tree/sgx-master) based on [substraTEE project](https://github.com/scs/substraTEE). 

## Building

Install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh
```

Install required tools:

```bash
./scripts/init.sh
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

Offchain-indexing is needed for the custom rpc `communities_getAll`. If you don't want it, omit the flag.
`--rpc-methods unsafe` is needed for the bazaar's business and offering aggregation rpcs.

Additional CLI usage options are available and may be shown by running `./target/release/encointer-node-notee --help`.

## Run Testnet Gesell Node
Join our testnet as a full node with 

```bash
./target/release/encointer-node-notee --chain gesellv2SpecRaw.json --name giveyournodeaname
```

## CLI client
We currently have limited support for the [polkadot-js apps](https://polkadot.js.org/apps) UI. Encointer comes with a cli application instead that supports all interactions with the chain

### Run Client

```
encointer-node/client> cargo build --release
encointer-node/client> ../target/release/encointer-client-notee transfer //Alice 5GziKpBELV7fuYNy7quQfWGgVARn8onchS86azuPQkFj9nEZ 1000000
encointer-node/client> ../target/release/encointer-client-notee list_participant_registry
encointer-node/client> ../target/release/encointer-client-notee list_meetup_registry
encointer-node/client> ../target/release/encointer-client-notee list_witnesses_registry
encointer-node/client> ../target/release/encointer-client-notee --help
``` 
The master of ceremony can play fast-forward for demo purposes (ceremonies only happen ~monthly. not good for demos)
```
encointer-node/client> ./encointer-client-notee next_phase
```

To run a full demo (you may need to fix ports in the scripts if you change them):
```
encointer-node/client> ./bootstrap_demo_community.sh
```

### Grow Bot Community

Assuming a local node is running with default ports:
```
pip3 install random_word pyproj geojson
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
This allows you to explore chain state but it doesn't support all types of extrinsic parameters needed. Use our CLI client instead.

## Mobile App

The PoC1 Android App doesn't work with this release anymore, but you can watch progress at [encointer-app](https://github.com/encointer/encointer-app)

## Dev-Remarks

### Benchmarking
For benchmarking a new pallet you need to do the following:

1. Add the new pallet to be benchmarked to the `define_benchmarks!` macro in the runtime.
2. Make sure you enable the pallet's benchmark by enabling its runtime-benchmark feature in the runtime's toml.
3. Compile the node with `--features runtime-benchmark`
4. Add it to the benchmark script: `./scripts/runtime_benchmark.sh`

This will automatically generate the new/updated weight file in `./runtime/src/weights`.