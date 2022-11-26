#!/usr/bin/env bash
set -euo pipefail

echo "Register test businesses"


CURRENT_DIR=$(pwd)

cd "$CLIENT_DIR"

python "$CLIENT_DIR/bot-community.py" --client $CLIENT_BIN init
python "$CLIENT_DIR/register-businesses.py" --client $CLIENT_BIN

cd "$CURRENT_DIR"