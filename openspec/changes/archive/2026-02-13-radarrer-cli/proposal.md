## Why

There is no first-class Rust CLI tool for managing Radarr instances. Existing options are limited to the web UI or ad-hoc curl/script solutions. A proper CLI enables automation, scripting, headless server management, and advanced operations like disk-aware cleanup that the web UI doesn't support natively. Building in Rust gives us a single static binary with no runtime dependencies, fast startup, and type safety against the full Radarr v3 API (237 endpoints, 137 schemas).

## What Changes

- New Rust binary crate `radarrer` providing a subcommand-based CLI (`clap`) for the full Radarr v3 API
- Generated HTTP client from the Radarr OpenAPI v3 spec (`openapi.json`) using `progenitor` or similar OpenAPI codegen
- Strongly-typed request/response models derived from the 137 API schemas
- Connection management: base URL, API key, optional TLS settings, configurable via CLI flags, env vars, or config file (`~/.config/radarrer/config.toml`)
- Output formatting: human-readable tables (default), JSON, and quiet/scriptable modes
- Advanced disk management commands beyond the base API:
  - `radarrer cleanup` — delete movies when total disk usage exceeds a threshold
  - Deletion strategies: `--strategy oldest-added` (oldest by date added), `--strategy least-activity` (fewest history events), `--strategy oldest-activity` (longest time since last history event)
  - Dry-run mode (`--dry-run`) to preview what would be deleted
  - Configurable size threshold (`--max-size 2TB`)

## Capabilities

### New Capabilities
- `api-client`: Generated HTTP client and typed models from Radarr OpenAPI spec; connection/auth config management
- `cli-interface`: Subcommand structure mapping to all Radarr API resource groups (movie, queue, history, system, config, etc.) with output formatting
- `disk-cleanup`: Advanced disk management — threshold-based movie deletion with configurable strategies and dry-run support

### Modified Capabilities

## Impact

- **New crate**: `radarrer` binary crate with `Cargo.toml`, generated client code, CLI entrypoint
- **Dependencies**: `clap` (CLI), `reqwest`/`progenitor-client` (HTTP), `serde`/`serde_json` (serialization), `tokio` (async runtime), `tabled` or `comfy-table` (table output), `toml` (config), `directories` (XDG paths)
- **External API**: Radarr v3 REST API — read and write operations; the cleanup feature performs destructive deletes
- **Build**: Nix flake already provides Rust toolchain; `openapi.json` is checked into the repo as the codegen source
