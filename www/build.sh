#!/usr/bin/env bash
set -euo pipefail

site_root=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
cd "$site_root/.."
wasm-pack build --target web --release --out-dir "$site_root/dist/pkg" --no-default-features --features web
cp -rf "$site_root/frontend/." "$site_root/dist/"
