#!/usr/bin/env bash
set -euo pipefail

echo "Bootstrapping demo community and running tests"

python bootstrap_demo_community.py --client $CLIENT_BIN --test