#!/bin/bash

# Create `WeightInfo` implementations for all the pallets and store it in the weight module of the `encointer-runtime`.
#
# NOTE: uncommenting the following line
#
# `--template="$SCRIPTS_DIR"/frame-weight-template-full-info.hbs`
#
# creates a extended weight.rs file, which does not only  contain benchmarked weights and implements `WeightInfo`.
# It does also:
# * create the `WeightInfo` definition
# * implement `WeightInfo` for `()`, to be used for tests
# * Create a `EncointerWeight` struct, which contains sensible weights that could be used in another runtime
#
# The generated output of the extended file is intended to be copied to the respective pallet's crate and used as the
# `<pallet>::Config::WeightInfo` declaration.


# use absolute paths to call this from wherever we want
SCRIPTS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
PROJ_ROOT="$(dirname "$SCRIPTS_DIR")"

NODE=${1:-target/release/encointer-node-notee}
CHAIN_SPEC=${2:-dev}
WEIGHT_OUTPUT_DIR=${3:-runtime/src/weights}

echo "Running benchmarks for all pallets:"
echo "NODE:               ${NODE}"
echo "CHAIN_SPEC:         ${CHAIN_SPEC}"
echo "WEIGHT_OUTPUT_DIR:  ${WEIGHT_OUTPUT_DIR}"

mkdir -p "$WEIGHT_OUTPUT_DIR"

pallets=(
#  "frame_system" \
#  "pallet_balances" \
#  "pallet_encointer_balances"
#  "pallet_encointer_bazaar"
#  "pallet_encointer_ceremonies"
#  "pallet_encointer_communities"
  "pallet_encointer_scheduler"
)

for pallet in ${pallets[*]}; do
  echo benchmarking "$pallet"...

  $NODE \
  benchmark \
  --chain="$CHAIN_SPEC" \
  --steps=50 \
  --repeat=20 \
  --pallet="$pallet" \
  --extrinsic="*" \
  --execution=wasm \
  --wasm-execution=compiled \
  --heap-pages=4096 \
  --output="$WEIGHT_OUTPUT_DIR"/"$pallet".rs \
#  --template="$SCRIPTS_DIR"/frame-weight-template-full-info.hbs

done
