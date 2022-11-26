#!/usr/bin/env bash
set -euo pipefail

echo "Register test businesses"

python "$CLIENT_DIR/bot-community.py" --client $CLIENT_BIN init
python "$CLIENT_DIR/register-businesses.py" --client $CLIENT_BIN