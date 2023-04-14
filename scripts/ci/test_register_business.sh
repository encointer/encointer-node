#!/usr/bin/env bash
set -euo pipefail

echo "Register test businesses"


CURRENT_DIR=$(pwd)

cd "$CLIENT_DIR"

python bot-community.py --client $CLIENT_BIN init
python register-businesses.py --client $CLIENT_BIN

cd "$CURRENT_DIR"