#!/bin/bash
# Run Axiom Web Server
#
# Usage: ./scripts/run-web.sh [--release] [--port PORT]

set -e

cd "$(dirname "$0")/.."

BUILD_MODE=""
PORT=8080

while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            BUILD_MODE="--release"
            shift
            ;;
        --port)
            PORT="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

if [[ "$BUILD_MODE" == "--release" ]]; then
    echo "Building server in release mode..."
else
    echo "Building server in debug mode..."
fi

cargo build -p axiom-server $BUILD_MODE

export PORT=$PORT
export RUST_LOG="${RUST_LOG:-axiom_server=debug,tower_http=debug}"

echo ""
echo "Starting Axiom Server on http://localhost:$PORT"
echo "WebSocket: ws://localhost:$PORT/api/workspaces/:id/ws"
echo ""

if [[ "$BUILD_MODE" == "--release" ]]; then
    ./target/release/axiom-server
else
    ./target/debug/axiom-server
fi
