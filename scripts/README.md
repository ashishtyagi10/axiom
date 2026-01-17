# Axiom Run Scripts

## TUI (Terminal UI)

```bash
# Debug mode
./scripts/run-tui.sh

# Release mode (optimized)
./scripts/run-tui.sh --release
```

Controls:
- `Tab` - Switch panels
- `q` - Quit (in normal mode)
- `Ctrl+C` - Force quit
- `#claude <prompt>` - Invoke Claude Code
- `!<command>` - Run shell command

## Web Server

```bash
# Debug mode (default port 8080)
./scripts/run-web.sh

# Release mode
./scripts/run-web.sh --release

# Custom port
./scripts/run-web.sh --port 3000
```

Endpoints:
- `http://localhost:8080` - Web UI (landing page for now)
- `http://localhost:8080/api/health` - Health check
- `http://localhost:8080/api/workspaces` - List workspaces
- `ws://localhost:8080/api/workspaces/:id/ws` - WebSocket stream

## Run Both

```bash
# Terminal 1: TUI
./scripts/run-tui.sh

# Terminal 2: Web Server
./scripts/run-web.sh
```

## Environment Variables

```bash
# Web server logging
export RUST_LOG=axiom_server=debug,tower_http=debug

# Custom port
export PORT=3000
```
