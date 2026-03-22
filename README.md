# forge

**SparkForge CLI** - Monitor and manage SparkForge client sites from the command line.

SparkForge is a Milwaukee-based network solutions and AI automation company. We deploy and manage enterprise network infrastructure alongside intelligent AI agents for businesses. The `forge` CLI provides a unified status view of any SparkForge-managed client site.

## What It Does

Think of `forge` as a fast, beautiful `htop` for a managed network + AI stack. It gives you instant visibility into:

- **Network infrastructure** - Switches, access points, VLANs
- **Server health** - Rack servers with specs and connectivity status
- **AI agents** - Running OpenClaw agents with health endpoints
- **Uptime monitoring** - Historical availability with visual progress bars

```
$ forge status --site client-04

Acme Corp (client-04)
Milwaukee, WI

[network]  48-port managed switch ONLINE
[network]  6x enterprise APs ONLINE
[network]  VLANs: ops/guest/iot SEGMENTED
[server]   rack-01 32GB/1TB SSD ONLINE
[agents]   lead-enrichment RUNNING
[agents]   doc-processor RUNNING
[agents]   competitor-watch RUNNING
[monitor]  uptime 99.97% |||||||||| 42d
```

## Installation

### Quick Install (curl)

**Linux/macOS:**
```bash
curl -fsSL https://github.com/sparkforge/forge-cli/releases/latest/download/forge-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m) -o forge
chmod +x forge
sudo mv forge /usr/local/bin/
```

**macOS (Apple Silicon):**
```bash
curl -fsSL https://github.com/sparkforge/forge-cli/releases/latest/download/forge-macos-arm64 -o forge
chmod +x forge
sudo mv forge /usr/local/bin/
```

### Build from Source

```bash
# Requires Rust 1.70+
cargo install --git https://github.com/sparkforge/forge-cli

# Or clone and build locally
git clone https://github.com/sparkforge/forge-cli
cd forge-cli
cargo build --release
./target/release/forge --help
```

## Commands

### `forge status [--site <name>] [--watch]`

Display the status of all components at a site.

```bash
# Check a specific site
forge status --site client-04

# Watch mode (refreshes every 30 seconds)
forge status --site client-04 --watch

# Use FORGE_SITE environment variable
export FORGE_SITE=client-04
forge status
```

### `forge sites`

List all configured sites with health indicators.

```bash
$ forge sites

Configured Sites
────────────────────────────────────────
  ● client-04
  ● client-07
  ● client-12
```

### `forge site add`

Interactive wizard to add a new site configuration.

```bash
$ forge site add

Add New Site Configuration
────────────────────────────────────────

Site identifier (e.g., client-04): client-15
Display name (e.g., Acme Corp): MegaCorp Industries
Location (e.g., Milwaukee, WI): Chicago, IL
...
```

### `forge agents [--site <name>]`

Detailed view of AI agent status with last-run times and error counts.

```bash
$ forge agents --site client-04

Agent Status - client-04
────────────────────────────────────────────────────────────────

  ● lead-enrichment
    Status:    RUNNING
    Endpoint:  http://192.168.1.100:3001/health
    Last Run:  2024-01-15T10:30:00Z
    Errors:    0

  ● doc-processor
    Status:    RUNNING
    Endpoint:  http://192.168.1.100:3002/health
```

### `forge ping <host>`

Quick connectivity check with latency measurement.

```bash
$ forge ping 192.168.1.1
● 192.168.1.1 is reachable (12.34ms)

$ forge ping 10.0.0.99
● 10.0.0.99 is unreachable
```

## Configuration

Site configurations are stored as TOML files in `~/.config/forge/sites/`.

### Example: `~/.config/forge/sites/client-04.toml`

```toml
[site]
name = "client-04"
display_name = "Acme Corp"
location = "Milwaukee, WI"

[[network.switches]]
host = "192.168.1.1"
label = "48-port managed switch"
type = "http"  # or "ping" or "snmp"
health_url = "http://192.168.1.1/api/status"  # optional

[[network.access_points]]
count = 6
label = "enterprise APs"
hosts = ["192.168.1.10", "192.168.1.11"]  # subset to ping-check

[[network.vlans]]
names = ["ops", "guest", "iot"]

[[servers]]
host = "192.168.1.100"
label = "rack-01"
ram_gb = 32
storage = "1TB SSD"
health_url = "http://192.168.1.100:8080/health"  # optional

[[agents]]
name = "lead-enrichment"
health_url = "http://192.168.1.100:3001/health"

[[agents]]
name = "doc-processor"
health_url = "http://192.168.1.100:3002/health"

[[agents]]
name = "competitor-watch"
health_url = "http://192.168.1.100:3003/health"

[monitor]
uptime_url = "https://uptime.sparkforge.io/api/site/client-04"  # optional
```

### Config Field Reference

| Section | Field | Description |
|---------|-------|-------------|
| `site` | `name` | Unique site identifier |
| `site` | `display_name` | Human-readable name |
| `site` | `location` | Physical location (optional) |
| `network.switches` | `host` | IP or hostname |
| `network.switches` | `label` | Display label |
| `network.switches` | `type` | Check type: `http`, `ping`, or `snmp` |
| `network.switches` | `health_url` | HTTP health endpoint (optional) |
| `network.access_points` | `count` | Total number of APs |
| `network.access_points` | `label` | Display label |
| `network.access_points` | `hosts` | IPs to ping-check (optional) |
| `network.vlans` | `names` | List of VLAN names |
| `servers` | `host` | IP or hostname |
| `servers` | `label` | Display label |
| `servers` | `ram_gb` | RAM in GB (optional) |
| `servers` | `storage` | Storage description (optional) |
| `servers` | `health_url` | HTTP health endpoint (optional) |
| `agents` | `name` | Agent name |
| `agents` | `health_url` | Agent health endpoint |
| `monitor` | `uptime_url` | Uptime Kuma API endpoint (optional) |

## Health Check Logic

For each component, `forge` tries these checks in order:

1. **HTTP Health**: If `health_url` is provided, HTTP GET expecting 200
2. **TCP Connect**: Try ports 443, 80, 22, 8080
3. **ICMP Ping**: Fallback if TCP fails

All checks have a 3-second timeout and run concurrently for speed.

## Environment Variables

| Variable | Description |
|----------|-------------|
| `FORGE_SITE` | Default site name when `--site` not specified |

## Output Colors

| Category | Color |
|----------|-------|
| `[network]` | Cyan |
| `[server]` | Blue |
| `[agents]` | Green |
| `[monitor]` | Yellow |
| `ONLINE` / `RUNNING` | Bright Green |
| `OFFLINE` / `ERROR` | Bright Red |
| `DEGRADED` / `WARNING` | Yellow |

## License

MIT

## Authors

SparkForge Team - [sparkforge.io](https://sparkforge.io)
