#!/usr/bin/env bash
set -euo pipefail
export PYTHONUNBUFFERED=1

echo "grow community for 7 ceremony cycles with full feature coverage"


CURRENT_DIR=$(pwd)

cd "$CLIENT_DIR"

python bot-community.py --client $CLIENT_BIN init
python bot-community.py --client $CLIENT_BIN simulate --ceremonies 7
diff bot-stats.csv bot-stats-golden.csv

cd "$CURRENT_DIR"
