#!/usr/bin/env bash
set -euo pipefail

echo "grow community for 2 entire ceremony cycles"

python bot-community.py --client $CLIENT_BIN init
python bot-community.py --client $CLIENT_BIN test
diff bot-stats.csv &CLIENT_DIR/"bot-stats-golden.csv"