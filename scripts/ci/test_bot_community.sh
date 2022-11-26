#!/usr/bin/env bash
set -euo pipefail

echo "grow community for 2 entire ceremony cycles"


CURRENT_DIR=$(pwd)

cd "$CLIENT_DIR"

python bot-community.py --client $CLIENT_BIN init
python bot-community.py --client $CLIENT_BIN test
diff bot-stats.csv bot-stats-golden.csv

cd "$CURRENT_DIR"