# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

STM (SSH Tunnel Manager) is a Rust TUI application that provides VS Code-like SSH port forwarding directly in the terminal. It acts as a thin orchestration layer on top of OpenSSH, using ControlMaster sockets and `-O forward`/`-O cancel` for dynamic tunnel management. See `STM.prd` for the full product requirements document.

## Tech Stack

- **Language:** Rust
- **TUI:** Ratatui
- **Async:** Tokio
- **Config:** TOML via `toml` crate, serde/serde_json for persistence
- **SSH:** Wraps OpenSSH binaries (not an SSH library reimplementation)

## Commands

```bash
# Build
cargo build

# Run
cargo run

# Run tests
cargo test

# Run a single test
cargo test test_name

# Run tests in a specific module
cargo test module_name::

# Check compilation without building
cargo check

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt

# Format check (CI)
cargo fmt -- --check
```

## Architecture

STM does **not** reimplement SSH. It orchestrates OpenSSH processes via ControlMaster:

```
User TUI (Ratatui) → ssh -S socket -O forward/cancel → OpenSSH ControlMaster → Remote Server
```

### Planned Module Structure

```
src/
├── main.rs           # Entry point, CLI arg parsing
├── app.rs            # Global application state, event loop
├── ui/               # Ratatui rendering
│   ├── host_list.rs  # Left panel: SSH hosts
│   ├── tunnel_list.rs# Right panel: active tunnels
│   ├── add_modal.rs  # Tunnel creation modal
│   ├── status_bar.rs # Connection status
│   └── theme.rs      # Colors/styles
├── ssh/              # OpenSSH interaction
│   ├── config.rs     # ~/.ssh/config parser
│   ├── connection.rs # ControlMaster lifecycle
│   └── tunnel.rs     # -O forward / -O cancel operations
├── state/            # Persistence
│   ├── history.rs    # Recent hosts/tunnels
│   └── persistence.rs# Config/history file I/O
└── error.rs          # Error types
```

### Key Design Decisions

- **OpenSSH wrapper, not SSH library:** Uses `ssh -M -S <socket>` for ControlMaster connections and `ssh -O forward/cancel` for dynamic tunnel management. This preserves compatibility with the user's existing `~/.ssh/config`, keys, and proxies.
- **Socket location:** `~/.config/stm/sockets/%h-%p` (dedicated directory to avoid conflicts with user's own ControlMaster setup).
- **Config:** `~/.config/stm/config.toml` for settings, `~/.config/stm/history.json` for host/tunnel history.

### Core Data Flow

1. Parse `~/.ssh/config` to list available hosts
2. User selects host → spawn `ssh -M -S <socket> -N user@host` (ControlMaster)
3. User adds tunnel → run `ssh -S <socket> -O forward -L local:remote_host:remote_port user@host`
4. User toggles tunnel off → run `ssh -S <socket> -O cancel -L ...`
5. Connection check → `ssh -S <socket> -O check user@host`

### MVP Scope (v0.1)

Local tunnels (`-L`) only. Remote (`-R`) and Dynamic (`-D`) tunnels are v0.2. Multi-host simultaneous connections, auto-detect of remote ports, and tmux integration are v2.
