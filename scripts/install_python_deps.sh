#!/usr/bin/env bash
set -euo pipefail

python3 -m pip install --upgrade pip
pip install geojson pyproj RandomWords wonderwords requests flask substrate-interface click
