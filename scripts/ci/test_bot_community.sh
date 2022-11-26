#!/usr/bin/env bash
set -euo pipefail

echo "grow community for 2 entire ceremony cycles"

python "$CLIENT_DIR/bot-community.py" --client $CLIENT_BIN init
python "$CLIENT_DIR/bot-community.py" --client $CLIENT_BIN test
diff bot-stats.csv "$CLIENT_DIR/bot-stats-golden.csv"