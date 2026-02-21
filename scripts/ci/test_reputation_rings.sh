#!/usr/bin/env bash
set -euo pipefail

echo "=============================================="
echo "  Reputation Rings E2E Test"
echo "  Automatic ring computation & ring-VRF proofs"
echo "=============================================="
echo ""

# Get absolute path for the binary
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJ_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
CLI_BIN="${CLIENT_BIN:-$PROJ_ROOT/target/release/encointer-cli}"
CLI="$CLI_BIN -u ws://127.0.0.1 -p 9944"
CLIENT_DIR_DEFAULT="$PROJ_ROOT/cli"
CLIENT_DIR="${CLIENT_DIR:-$CLIENT_DIR_DEFAULT}"

echo "=== Step 1: Bootstrap a demo community (first ceremony) ==="
cd "$CLIENT_DIR"
python3 bootstrap_demo_community.py --client "$CLI_BIN" -u ws://127.0.0.1 -p 9944 -l --signer //Alice --test none
echo ""

# After bootstrap with --test none: cindex=2, phase=Registering.
# Ceremony 1 completed with reputation for Alice, Bob, Charlie.
# Rings were NOT auto-computed for ceremony 1 because the Assigning
# phase (cindex=1) had completed=0 and on_ceremony_phase_change returns early.

echo "=== Step 2: Get community ID and ceremony index ==="
CID=$($CLI community list | grep -v "number of" | head -1 | awk -F: '{print $1}')
echo "Using community: $CID"

if [ -z "$CID" ]; then
    echo "ERROR: Could not find community ID"
    exit 1
fi

CINDEX=$($CLI ceremony index)
echo "Current ceremony index: $CINDEX"
PHASE=$($CLI ceremony phase)
echo "Current phase: $PHASE"

echo "=== Step 3: Register Bandersnatch keys (auto-derived) ==="
# Keys must be registered BEFORE the Assigning phase so they are
# picked up by automatic ring computation.
$CLI account bandersnatch-pubkey register //Alice
echo "Registered Bandersnatch key for Alice"

$CLI account bandersnatch-pubkey register //Bob
echo "Registered Bandersnatch key for Bob"

$CLI account bandersnatch-pubkey register //Charlie
echo "Registered Bandersnatch key for Charlie"

echo "=== Step 4: Advance to Assigning phase (triggers auto ring computation) ==="
# We are in Registering phase with cindex=2.
# Transitioning to Assigning triggers on_ceremony_phase_change(Assigning)
# which queues ring computation for completed ceremony (cindex-1 = 1).
# Participants got reputation from ceremony 1 (bootstrap), and we just
# registered their Bandersnatch keys, so on_idle will build rings.
$CLI ceremony admin next-phase --signer //Alice
PHASE=$($CLI ceremony phase)
echo "Phase after advance: $PHASE"
if [ "$PHASE" != "Assigning" ]; then
    echo "ERROR: Expected Assigning phase, got $PHASE"
    exit 1
fi

# Poll for on_idle to process ring computation across several blocks.
# Ring computation needs ~11 steps (6 collection + 5 building).
echo "Waiting for on_idle ring computation..."
COMPLETED_CINDEX=$((CINDEX - 1))
for i in $(seq 1 60); do
    RINGS_OUTPUT=$($CLI personhood ring get --cid $CID --ceremony-index $COMPLETED_CINDEX 2>&1) || true
    if echo "$RINGS_OUTPUT" | grep -q "Level 1/5"; then
        echo "Rings computed after ${i}s"
        break
    fi
    if [ "$i" -eq 60 ]; then
        echo "ERROR: Ring computation timed out after 60s"
        exit 1
    fi
    sleep 1
done
echo "Ring-computed ceremony index: $COMPLETED_CINDEX"

echo "=== Step 5: Query auto-computed rings ==="
RINGS_OUTPUT=$($CLI personhood ring get --cid $CID --ceremony-index $COMPLETED_CINDEX)
echo "$RINGS_OUTPUT"

echo "=== Step 6: Verify rings ==="
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

echo "=== Step 7: Ring-VRF Proof of Personhood ==="
PROVE_OUTPUT=$($CLI personhood prove-ring-membership //Alice --cid $CID \
    --ceremony-index $COMPLETED_CINDEX --level 1 --sub-ring 0)
echo "$PROVE_OUTPUT"
SIGNATURE=$(echo "$PROVE_OUTPUT" | grep "^signature:" | awk '{print $2}')
[ -n "$SIGNATURE" ] || { echo "ERROR: prove-personhood failed"; exit 1; }

echo "=== Step 8: Verify ring-VRF proof ==="
VERIFY_OUTPUT=$($CLI personhood verify-ring-membership --cid $CID \
    --ceremony-index $COMPLETED_CINDEX --level 1 --sub-ring 0 \
    --signature $SIGNATURE)
echo "$VERIFY_OUTPUT"
echo "$VERIFY_OUTPUT" | grep -q "VALID" || { echo "ERROR: verify failed"; exit 1; }

echo "=== Step 9: Wrong context must fail ==="
$CLI personhood verify-ring-membership --cid $CID --ceremony-index 999 \
    --level 1 --sub-ring 0 --signature $SIGNATURE 2>&1 && {
    echo "ERROR: should have failed"; exit 1; } || true

echo ""
echo "=============================================="
echo "  Reputation Rings E2E Test PASSED"
echo "=============================================="
echo ""
echo "Summary:"
echo "  - Bandersnatch key registration before Assigning phase (auto-derived)"
echo "  - Rings auto-computed via on_idle during Assigning phase"
echo "  - 5 ring levels queried and verified"
echo "  - Ring nesting property confirmed"
echo "  - Ring-VRF prove and verify"
