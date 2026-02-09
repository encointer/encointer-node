#!/usr/bin/env bash
set -euo pipefail

echo "Testing offline payment functionality (ZK PoC)"

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

echo "=== Step 4: Register offline identities ==="
$CLI register-offline-identity //Alice --cid $CID
echo "Registered offline identity for Alice"

$CLI register-offline-identity //Bob --cid $CID
echo "Registered offline identity for Bob"

echo "=== Step 5: Verify offline identities ==="
ALICE_COMMITMENT=$($CLI get-offline-identity //Alice | grep "Commitment:" | awk '{print $2}')
echo "Alice commitment: $ALICE_COMMITMENT"

BOB_COMMITMENT=$($CLI get-offline-identity //Bob | grep "Commitment:" | awk '{print $2}')
echo "Bob commitment: $BOB_COMMITMENT"

if [ -z "$ALICE_COMMITMENT" ] || [ -z "$BOB_COMMITMENT" ]; then
    echo "ERROR: Failed to register offline identities"
    exit 1
fi

echo "=== Step 6: Generate offline payment data ==="
# Note: This generates the public inputs for ZK proof generation
# In a full implementation, this would include a Groth16 proof
$CLI generate-offline-payment --signer //Alice --to //Bob --amount 1.0 --cid $CID > /tmp/payment.json
echo "Generated payment data:"
cat /tmp/payment.json

# Verify the output has expected fields
if ! grep -q "commitment" /tmp/payment.json; then
    echo "ERROR: Payment data missing 'commitment' field"
    exit 1
fi

if ! grep -q "nullifier" /tmp/payment.json; then
    echo "ERROR: Payment data missing 'nullifier' field"
    exit 1
fi

if ! grep -q "sender" /tmp/payment.json; then
    echo "ERROR: Payment data missing 'sender' field"
    exit 1
fi

if ! grep -q "recipient" /tmp/payment.json; then
    echo "ERROR: Payment data missing 'recipient' field"
    exit 1
fi

echo ""
echo "=== Offline payment tests PASSED ==="
echo ""
echo "Summary:"
echo "  - Offline identity registration: PASSED"
echo "  - Offline identity retrieval: PASSED"
echo "  - Payment data generation: PASSED"
echo ""
echo "Note: Full payment settlement requires:"
echo "  1. A verification key to be set via sudo"
echo "  2. A valid Groth16 proof (not included in PoC)"
echo "  3. Community currency balance for the sender"
