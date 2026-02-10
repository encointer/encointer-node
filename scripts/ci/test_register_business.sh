#!/usr/bin/env bash
set -euo pipefail

echo "Register test businesses"


CURRENT_DIR=$(pwd)

cd "$CLIENT_DIR"

python bot-community.py --client $CLIENT_BIN -l init
python register-random-businesses-and-offerings.py --client $CLIENT_BIN -l

cd "$CURRENT_DIR"
