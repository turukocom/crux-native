#!/usr/bin/env bash
set -euo pipefail
# Runs Crux-MIR ONLY on a native x86_64 Linux host. Refuses Rosetta/arm64.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"; cd "$ROOT"
if [[ "$(uname -s)" != "Linux" || "$(uname -m)" != "x86_64" ]]; then
  echo "REFUSED: native x86_64 Linux required; got $(uname -s)/$(uname -m)."
  echo "Apple Silicon/Rosetta is not supported (rustc SIGSEGV)."
  exit 2
fi
command -v docker >/dev/null || { echo "FAIL: docker yok" >&2; exit 3; }
docker run --rm \
  -v "$ROOT/formal/crux:/work:rw" -w /work \
  ghcr.io/galoisinc/crux-mir:0.12 \
  crux-mir /work/simple.rs
