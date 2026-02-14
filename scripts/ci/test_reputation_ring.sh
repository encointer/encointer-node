#!/usr/bin/env bash
set -euo pipefail

echo "=============================================="
echo "  Reputation Ring E2E Test"
echo "  Bandersnatch key registration & ring computation"
echo "=============================================="
echo ""

# Get absolute path for the binary
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJ_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
CLI_BIN="${CLIENT_BIN:-$PROJ_ROOT/target/release/encointer-client-notee}"
CLI="$CLI_BIN -u ws://127.0.0.1 -p 9944"
CLIENT_DIR_DEFAULT="$PROJ_ROOT/client"
CLIENT_DIR="${CLIENT_DIR:-$CLIENT_DIR_DEFAULT}"

echo "=== Step 1: Bootstrap a demo community ==="
cd "$CLIENT_DIR"
python3 bootstrap_demo_community.py --client "$CLI_BIN" -u ws://127.0.0.1 -p 9944 -l --signer //Alice --test none
echo ""

echo "=== Step 2: Get community ID and ceremony index ==="
CID=$($CLI list-communities | grep -v "number of" | head -1 | awk -F: '{print $1}')
echo "Using community: $CID"

if [ -z "$CID" ]; then
    echo "ERROR: Could not find community ID"
    exit 1
fi

CINDEX=$($CLI get-cindex)
echo "Current ceremony index: $CINDEX"

# The ceremony that just completed is CINDEX - 1
COMPLETED_CINDEX=$((CINDEX - 1))
echo "Completed ceremony index: $COMPLETED_CINDEX"

if [ "$COMPLETED_CINDEX" -lt 1 ]; then
    echo "ERROR: No completed ceremony found"
    exit 1
fi

echo "=== Step 3: Register Bandersnatch keys ==="
# Generate deterministic 32-byte keys for testing
ALICE_KEY="0101010101010101010101010101010101010101010101010101010101010101"
BOB_KEY="0202020202020202020202020202020202020202020202020202020202020202"
CHARLIE_KEY="0303030303030303030303030303030303030303030303030303030303030303"

$CLI register-bandersnatch-key //Alice --key $ALICE_KEY
echo "Registered Bandersnatch key for Alice"

$CLI register-bandersnatch-key //Bob --key $BOB_KEY
echo "Registered Bandersnatch key for Bob"

$CLI register-bandersnatch-key //Charlie --key $CHARLIE_KEY
echo "Registered Bandersnatch key for Charlie"

echo "=== Step 4: Initiate ring computation ==="
$CLI initiate-rings //Alice --cid $CID --ceremony-index $COMPLETED_CINDEX
echo "Ring computation initiated"

echo "=== Step 5: Run ring computation steps ==="
# Ring computation needs:
# - 5 or 6 steps for member collection (scanning last 5 ceremonies + transition)
# - 5 steps for ring building (levels 5 down to 1)
# Total: up to 11 steps. We loop until done or max 15 attempts.
for i in $(seq 1 15); do
    echo "  Step $i..."
    OUTPUT=$($CLI continue-ring-computation //Alice 2>&1) || true
    echo "  $OUTPUT"
    if echo "$OUTPUT" | grep -q "NoComputationPending\|ComputationAlreadyDone"; then
        echo "Ring computation completed after $i steps"
        break
    fi
done

echo "=== Step 6: Query rings ==="
RINGS_OUTPUT=$($CLI get-rings --cid $CID --ceremony-index $COMPLETED_CINDEX)
echo "$RINGS_OUTPUT"

echo "=== Step 7: Verify rings ==="
# Check that at least level 1 has members
LEVEL1_COUNT=$(echo "$RINGS_OUTPUT" | grep "Level 1/5" | grep -oP '\d+ members' | grep -oP '\d+')
if [ -z "$LEVEL1_COUNT" ] || [ "$LEVEL1_COUNT" -eq 0 ]; then
    echo "ERROR: Level 1/5 ring has no members"
    exit 1
fi
echo "Level 1/5 has $LEVEL1_COUNT members"

# Check that ring nesting holds: level N+1 count <= level N count
PREV_COUNT=$LEVEL1_COUNT
for level in 2 3 4 5; do
    COUNT=$(echo "$RINGS_OUTPUT" | grep "Level $level/5" | grep -oP '\d+ members' | grep -oP '\d+')
    if [ -z "$COUNT" ]; then
        COUNT=0
    fi
    if [ "$COUNT" -gt "$PREV_COUNT" ]; then
        echo "ERROR: Ring nesting violated: level $level/5 ($COUNT) > level $((level-1))/5 ($PREV_COUNT)"
        exit 1
    fi
    echo "Level $level/5 has $COUNT members (nested correctly)"
    PREV_COUNT=$COUNT
done

echo ""
echo "=============================================="
echo "  Reputation Ring E2E Test PASSED"
echo "=============================================="
echo ""
echo "Summary:"
echo "  - Bandersnatch key registration for 3 accounts"
echo "  - Ring computation initiated and completed"
echo "  - 5 ring levels queried and verified"
echo "  - Ring nesting property confirmed"
