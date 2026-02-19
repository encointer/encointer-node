#!/usr/bin/env bash
set -euo pipefail
export PYTHONUNBUFFERED=1

echo "grow community for 7 ceremony cycles with full feature coverage"


CURRENT_DIR=$(pwd)

cd "$CLIENT_DIR"

python bot-community.py --client $CLIENT_BIN init
python bot-community.py --client $CLIENT_BIN simulate --ceremonies 7
if ! diff bot-stats.csv bot-stats-golden.csv; then
  echo "⚠ WARNING: bot-stats.csv differs from golden file (see diff above)"
fi

cd "$CURRENT_DIR"
