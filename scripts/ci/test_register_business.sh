#!/usr/bin/env bash
set -euo pipefail

echo "Register test businesses"

python bot-community.py --client $CLIENT_BIN init
python register-businesses.py --client $CLIENT_BIN