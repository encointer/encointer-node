#!/usr/bin/env bash
set -euo pipefail

echo "Testing offline payment functionality"

CLI=$CLIENT_BIN

echo "=== Step 1: Create test accounts ==="
$CLI new-account --seed //Alice
$CLI new-account --seed //Bob
$CLI new-account --seed //Charlie

echo "=== Step 2: Fund accounts ==="
$CLI faucet //Alice //Bob //Charlie

echo "=== Step 3: Create a test community ==="
cd "$CLIENT_DIR"
CID=$($CLI new-community test-data/leu.json --signer //Alice | tail -1)
echo "Created community: $CID"

echo "=== Step 4: Issue community currency to Alice ==="
# Use sudo to issue currency directly (for testing)
# The node should be started with --dev which has Alice as sudo

echo "=== Step 5: Register offline identities ==="
$CLI register-offline-identity //Alice --cid $CID
echo "Registered offline identity for Alice"

$CLI register-offline-identity //Bob --cid $CID
echo "Registered offline identity for Bob"

echo "=== Step 6: Verify offline identities ==="
ALICE_COMMITMENT=$($CLI get-offline-identity //Alice | grep "Commitment:" | awk '{print $2}')
echo "Alice commitment: $ALICE_COMMITMENT"

BOB_COMMITMENT=$($CLI get-offline-identity //Bob | grep "Commitment:" | awk '{print $2}')
echo "Bob commitment: $BOB_COMMITMENT"

if [ -z "$ALICE_COMMITMENT" ] || [ -z "$BOB_COMMITMENT" ]; then
    echo "ERROR: Failed to register offline identities"
    exit 1
fi

echo "=== Step 7: Generate offline payment proof ==="
# Note: For a full integration test, we'd need to issue balance to Alice first
# For now, this tests the proof generation mechanism
$CLI generate-offline-payment --signer //Alice --to //Bob --amount 1.0 --cid $CID > /tmp/payment.json
echo "Generated payment proof:"
cat /tmp/payment.json

# Verify the proof file was created and has expected fields
if ! grep -q "proof" /tmp/payment.json; then
    echo "ERROR: Payment proof missing 'proof' field"
    exit 1
fi

if ! grep -q "nullifier" /tmp/payment.json; then
    echo "ERROR: Payment proof missing 'nullifier' field"
    exit 1
fi

echo ""
echo "=== Offline payment tests PASSED ==="
echo ""
echo "Note: Full payment settlement requires community currency balance."
echo "The proof generation and identity registration have been verified."
