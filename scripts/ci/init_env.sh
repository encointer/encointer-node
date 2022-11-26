#!/bin/bash
set -euo pipefail

# script that sets the correct environment variables to execute other scripts

export SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
export PROJ_ROOT="$(dirname "$SCRIPT_DIR")"
export CLIENT_DIR="$PROJ_ROOT/client"
export CLIENT_BIN="$PROJ_ROOT/target/release/encointer-client-notee"
export NODE_BIN="$PROJ_ROOT/target/release/encointer-node-notee"


echo "Set environment variables:"
echo "  BASH_SCRIPT_DIR: $SCRIPT_DIR"
echo "  PROJ_ROOT: $PROJ_ROOT"
echo "  Client Directory: $CLIENT_DIR"
echo "  Cleint Binary: $CLIENT_BIN"
echo "  Node binary: $NODE_BIN"
