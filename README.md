# STM - SSH Tunnel Manager

A terminal UI for managing SSH tunnels, inspired by VS Code's port forwarding panel. STM wraps OpenSSH's ControlMaster to provide dynamic local port forwarding (`-L`) without modifying your SSH config.

## Install

### From Binary Release (Recommended)

Download the latest release for your platform from [GitHub Releases](https://github.com/yourusername/stm/releases):

```bash
# macOS (Apple Silicon)
curl -L https://github.com/yourusername/stm/releases/download/v0.1.0/stm-0.1.0-aarch64-apple-darwin.tar.gz | tar xz
sudo mv stm-0.1.0-aarch64-apple-darwin/stm /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/yourusername/stm/releases/download/v0.1.0/stm-0.1.0-x86_64-apple-darwin.tar.gz | tar xz
sudo mv stm-0.1.0-x86_64-apple-darwin/stm /usr/local/bin/

# Linux
curl -L https://github.com/yourusername/stm/releases/download/v0.1.0/stm-0.1.0-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv stm-0.1.0-x86_64-unknown-linux-gnu/stm /usr/local/bin/

# Windows (PowerShell)
$release = "0.1.0"
Invoke-WebRequest https://github.com/yourusername/stm/releases/download/v$release/stm-$release-x86_64-pc-windows-msvc.zip -OutFile stm.zip
Expand-Archive stm.zip
Move-Item stm/stm-0.1.0-x86_64-pc-windows-msvc/stm.exe C:\Windows\System32\
```

### From Cargo

```bash
cargo install --path .
```

### Build from Source

```bash
git clone <repo-url> && cd stm
cargo build --release
# Binary at target/release/stm
```

### Requirements

Requires OpenSSH (`ssh`) on your PATH.

## Quick Start

```bash
stm
```

1. Select a host from your `~/.ssh/config` with `j`/`k` and press `Enter` to connect
2. Press `a` to add a tunnel (e.g. local 5432 -> localhost:5432)
3. Toggle tunnels on/off with `Space`, delete with `d`
4. Press `r` to restore previously saved tunnels for a host

## CLI Options

```
stm [OPTIONS]

Options:
  --ssh-config <PATH>   Path to SSH config file (overrides config.toml)
  --connect <HOST>      Auto-connect to a host on startup
  -h, --help            Print help
  -V, --version         Print version
```

## Keyboard Shortcuts

| Key         | Action                          |
|-------------|---------------------------------|
| `j` / `Down`    | Navigate down                    |
| `k` / `Up`      | Navigate up                      |
| `Enter`         | Connect to selected host         |
| `Tab` / `h`/`l` / `←`/`→` | Switch panel (Hosts/Tunnels) |
| `/`            | Search hosts                 |
| `a`            | Add tunnel                   |
| `Space`        | Toggle tunnel on/off         |
| `d`            | Delete tunnel                |
| `r`            | Restore saved tunnels        |
| `x`            | Disconnect                   |
| `?`            | Show help                    |
| `q` / `Esc`   | Quit                         |
| `Ctrl+C`       | Quit                         |

### In Add Tunnel Modal

| Key         | Action              |
|-------------|---------------------|
| `Tab`       | Next field           |
| `Enter`     | Submit               |
| `Esc`       | Cancel               |

## Configuration

STM reads configuration from `~/.config/stm/config.toml`. See [`config.example.toml`](config.example.toml) for all options.

```toml
[general]
# ssh_config_path = "~/.ssh/config"
# socket_dir = "~/.config/stm/sockets"
auto_restore = false
max_recent_hosts = 10

[ui]
show_all_hosts = true
```

Connection history and saved tunnels are persisted in `~/.config/stm/history.json`.

## How It Works

STM does not reimplement SSH. It orchestrates OpenSSH processes:

1. **Connect**: `ssh -M -S <socket> -N user@host` (ControlMaster)
2. **Add tunnel**: `ssh -S <socket> -O forward -L local:host:remote user@host`
3. **Remove tunnel**: `ssh -S <socket> -O cancel -L local:host:remote user@host`
4. **Health check**: `ssh -S <socket> -O check user@host` (periodic, ~10s)
5. **Disconnect**: `ssh -S <socket> -O exit user@host`

Sockets are stored in `~/.config/stm/sockets/` to avoid conflicts with your own ControlMaster setup.

## Limitations

- **Local tunnels only** (`-L`). Remote (`-R`) and dynamic (`-D`) tunnels are planned for v0.2.
- Single host connection at a time. Multi-host is planned for v2.
- Requires OpenSSH on PATH (not a built-in SSH implementation).
- `Include` directives in SSH config support simple globs only.

## License

MIT
