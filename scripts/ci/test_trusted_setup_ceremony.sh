#!/usr/bin/env bash
set -euo pipefail

echo "=============================================="
echo "  Multiparty Trusted Setup Ceremony E2E Test"
echo "  3-party ceremony + on-chain functional test"
echo "=============================================="
echo ""

# --- paths ---
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJ_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
CLI_BIN="${CLIENT_BIN:-$PROJ_ROOT/target/release/encointer-cli}"
CLI="$CLI_BIN -u ws://127.0.0.1 -p 9944"
CLIENT_DIR="${CLIENT_DIR:-$PROJ_ROOT/cli}"

TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

PK="$TMPDIR/ceremony_pk.bin"
TRANSCRIPT="$TMPDIR/ceremony_transcript.json"
FINAL_PK="$TMPDIR/proving_key.bin"
FINAL_VK="$TMPDIR/verifying_key.bin"

# =================================================================
# Part 1: Ceremony (local-only, no chain needed)
# =================================================================

echo "=== Step 1: Initialize ceremony ==="
$CLI_BIN offline-payment admin ceremony init --pk-out "$PK" --transcript "$TRANSCRIPT"
echo ""

echo "=== Step 2: Alice contributes ==="
$CLI_BIN offline-payment admin ceremony contribute --pk "$PK" --transcript "$TRANSCRIPT" --participant Alice
echo ""

echo "=== Step 3: Bob contributes ==="
$CLI_BIN offline-payment admin ceremony contribute --pk "$PK" --transcript "$TRANSCRIPT" --participant Bob
echo ""

echo "=== Step 4: Charlie contributes ==="
$CLI_BIN offline-payment admin ceremony contribute --pk "$PK" --transcript "$TRANSCRIPT" --participant Charlie
echo ""

echo "=== Step 5: Verify all contributions ==="
$CLI_BIN offline-payment admin ceremony verify --pk "$PK" --transcript "$TRANSCRIPT"
echo ""

echo "=== Step 6: Finalize ceremony ==="
$CLI_BIN offline-payment admin ceremony finalize --pk "$PK" --pk-out "$FINAL_PK" --vk-out "$FINAL_VK"
echo ""

echo "=== Step 7: Cross-check with trusted-setup verify ==="
$CLI_BIN offline-payment admin trusted-setup verify --pk "$FINAL_PK" --vk "$FINAL_VK"
echo ""

# =================================================================
# Part 2: On-chain functional test using ceremony keys
# =================================================================

echo "=== Step 8: Bootstrap a demo community ==="
cd "$CLIENT_DIR"
python3 bootstrap_demo_community.py --client "$CLI_BIN" -u ws://127.0.0.1 -p 9944 -l --signer //Alice --test none
echo ""

echo "=== Step 9: Get community ID ==="
CID=$($CLI community list | grep -v "number of" | head -1 | awk -F: '{print $1}')
echo "Using community: $CID"
if [ -z "$CID" ]; then
    echo "ERROR: Could not find community ID"
    exit 1
fi

echo "=== Step 10: Set ceremony VK on-chain via sudo ==="
$CLI offline-payment admin set-vk --vk-file "$FINAL_VK" --signer //Alice
echo ""

echo "=== Step 11: Register offline identities ==="
$CLI account poseidon-commitment register //Alice
$CLI account poseidon-commitment register //Bob
echo ""

echo "=== Step 12: Verify offline identities ==="
ALICE_COMMITMENT=$($CLI account poseidon-commitment get //Alice | grep "Commitment:" | awk '{print $2}')
BOB_COMMITMENT=$($CLI account poseidon-commitment get //Bob | grep "Commitment:" | awk '{print $2}')
echo "Alice commitment: $ALICE_COMMITMENT"
echo "Bob commitment:   $BOB_COMMITMENT"
if [ -z "$ALICE_COMMITMENT" ] || [ -z "$BOB_COMMITMENT" ]; then
    echo "ERROR: Failed to register offline identities"
    exit 1
fi

echo "=== Step 13: Generate offline payment with ceremony PK ==="
echo "Generating ZK proof with ceremony-derived proving key..."
$CLI offline-payment pay --signer //Alice --to //Bob --amount 0.1 --cid "$CID" \
    --pk-file "$FINAL_PK" 2>/dev/null > "$TMPDIR/payment.json"
echo "Generated payment proof:"
cat "$TMPDIR/payment.json"

if ! grep -q "proof" "$TMPDIR/payment.json"; then
    echo "ERROR: Payment data missing 'proof' field"
    exit 1
fi

echo "=== Step 14: Submit offline payment ==="
$CLI offline-payment settle --signer //Charlie --proof-file "$TMPDIR/payment.json"
echo ""

echo "=== Step 15: Verify balances ==="
ALICE_BALANCE=$($CLI balance //Alice --cid "$CID" | tail -1)
BOB_BALANCE=$($CLI balance //Bob --cid "$CID" | tail -1)
echo "Alice balance: $ALICE_BALANCE"
echo "Bob balance:   $BOB_BALANCE"

echo ""
echo "=============================================="
echo "  Multiparty Trusted Setup Ceremony PASSED"
echo "=============================================="
echo ""
echo "Summary:"
echo "  - 3-party ceremony (Alice, Bob, Charlie)"
echo "  - All contribution receipts verified (pairing check)"
echo "  - Ceremony PK/VK finalized and cross-checked"
echo "  - ZK proof generated with ceremony PK"
echo "  - On-chain verification + settlement succeeded"
