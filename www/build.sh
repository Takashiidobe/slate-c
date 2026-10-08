#!/usr/bin/env bash
set -euo pipefail

site_root=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
cd "$site_root/.."
python3 "$site_root/build.py"
