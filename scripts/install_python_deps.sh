#!/usr/bin/env bash
set -euo pipefail

python -m pip install --upgrade pip
pip install geojson pyproj RandomWords wonderwords requests flask substrate-interface click
