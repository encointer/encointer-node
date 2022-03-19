#!/bin/bash

# Create `WeightInfo` implementations for all the pallets and store it in the weight module of the `encointer-runtime`.

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
  "pallet_encointer_ceremonies"
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

done
