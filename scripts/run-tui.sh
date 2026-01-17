#!/bin/bash
# Run Axiom TUI (Terminal User Interface)
#
# Usage: ./scripts/run-tui.sh [--release]

set -e

cd "$(dirname "$0")/.."

BUILD_MODE=""
if [[ "$1" == "--release" ]]; then
    BUILD_MODE="--release"
    echo "Building TUI in release mode..."
else
    echo "Building TUI in debug mode..."
fi

cargo build -p axiom-tui $BUILD_MODE

if [[ "$BUILD_MODE" == "--release" ]]; then
    ./target/release/axiom-tui
else
    ./target/debug/axiom-tui
fi
