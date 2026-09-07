## Context

Radarr is a movie PVR application exposing a comprehensive REST API (v3) with 237 endpoints across 50 resource groups and 137 typed schemas. The OpenAPI spec is available at `openapi.json` in the repo root. There is no existing Rust client or CLI for this API. The project starts from scratch — no existing code to integrate with.

The Nix flake provides Rust stable (1.93+) and Node.js 20. The binary should be self-contained with no runtime dependencies beyond network access to a Radarr instance.

## Goals / Non-Goals

**Goals:**
- Complete coverage of all 237 Radarr v3 API endpoints via typed Rust client
- Ergonomic CLI with subcommands grouped by resource (movie, queue, system, etc.)
- Multiple output formats: human-readable tables, JSON, quiet mode
- Config file support (`~/.config/radarrer/config.toml`) with env var and flag overrides
- Disk cleanup feature: threshold-based movie deletion with multiple strategies
- Dry-run support for all destructive operations

**Non-Goals:**
- Real-time event streaming / SignalR websocket support (can be added later)
- A TUI / interactive mode (this is a non-interactive CLI)
- Being a general-purpose *arr CLI (Sonarr, Lidarr, etc.) — Radarr only
- GUI or web interface
- Plugin/extension system

## Decisions

### D1: OpenAPI codegen approach — `progenitor`

Generate the HTTP client and all request/response types directly from `openapi.json` using [progenitor](https://github.com/oxidecomputer/progenitor) (Oxide's OpenAPI client generator for Rust).

**Rationale:** Progenitor produces idiomatic async Rust with `reqwest` + `serde`, generates builder-pattern request methods, and handles the full OpenAPI 3.0 spec. It's actively maintained and produces compile-time-checked code. The alternative `openapi-generator` produces less idiomatic Rust and requires Java.

**Alternative considered:** Hand-writing the client. Rejected because 237 endpoints with 137 schemas would be error-prone and a maintenance burden when the API evolves.

**Implementation:** Use `progenitor-macro` for build-time generation via `build.rs`, producing a `radarr_client` module. If the macro approach hits limitations with the spec size, fall back to `progenitor-cli` for offline generation into a `generated/` directory.

### D2: CLI framework — `clap` derive

Use `clap` v4 with derive macros for the command structure. Each Radarr resource group maps to a top-level subcommand with CRUD sub-subcommands.

**Rationale:** `clap` is the de facto standard, supports derive for type-safe arg parsing, generates shell completions, and handles complex nested subcommands well.

**Alternative considered:** `argh` (simpler, but no shell completions or rich help formatting).

### D3: Project structure — single crate, module-per-resource

```
src/
  main.rs              # Entry point, clap dispatch
  config.rs            # Config file, env, CLI flag merging
  client.rs            # Generated client re-export + auth wrapper
  output.rs            # Table/JSON/quiet formatting
  commands/
    mod.rs
    movie.rs           # movie subcommands
    queue.rs           # queue subcommands
    history.rs         # history subcommands
    system.rs          # system/health/diskspace/backup
    collection.rs      # collections
    config_cmd.rs      # config get/set subcommands
    indexer.rs         # indexer CRUD
    download_client.rs # download client CRUD
    notification.rs    # notification CRUD
    import_list.rs     # import list CRUD
    quality.rs         # quality profiles & definitions
    custom_format.rs   # custom formats
    tag.rs             # tags
    blocklist.rs       # blocklist
    release.rs         # releases
    calendar.rs        # calendar
    cleanup.rs         # disk cleanup (advanced feature)
    ...                # remaining resource groups
  generated/           # progenitor output (if using CLI codegen)
```

**Rationale:** A single crate keeps things simple for a CLI tool. Module-per-resource keeps files manageable. The `commands/` directory mirrors the CLI subcommand tree.

### D4: Configuration layering

Priority (highest to lowest):
1. CLI flags (`--url`, `--api-key`)
2. Environment variables (`RADARR_URL`, `RADARR_API_KEY`)
3. Config file (`~/.config/radarrer/config.toml`)

```toml
[connection]
url = "http://localhost:7878"
api_key = "abc123"

[output]
format = "table"  # table | json | quiet

[cleanup]
default_strategy = "oldest-added"
default_max_size = "2TB"
```

**Rationale:** This follows standard CLI conventions (12-factor app). The config file provides defaults; env vars are good for CI/containers; flags override everything.

### D5: Disk cleanup architecture

The cleanup command is a composite operation built on top of the generated client:

1. Query `GET /api/v3/diskspace` to get current usage
2. If usage exceeds threshold, query `GET /api/v3/movie` to get all movies with sizes
3. Query `GET /api/v3/history/movie` per movie for activity data (batched)
4. Sort movies by selected strategy
5. Iterate through sorted list, accumulating freed space until under threshold
6. Delete each movie via `DELETE /api/v3/movie/{id}` (with `deleteFiles=true`)

Strategies:
- **oldest-added**: Sort by `added` date ascending (oldest first)
- **least-activity**: Sort by count of history events ascending (fewest first)
- **oldest-activity**: Sort by most recent history event date ascending (longest idle first)

**Rationale:** Building on the generated client means cleanup automatically benefits from typed models and auth. The strategy pattern is extensible for future strategies.

### D6: Output formatting

Use `comfy-table` for human-readable tables. Each command module defines a `format_output()` function that takes the API response and the output mode enum.

**Rationale:** `comfy-table` handles terminal width, Unicode, and alignment. It's lighter than `tabled` and well-maintained. JSON output is just `serde_json::to_string_pretty`. Quiet mode outputs only IDs or counts.

### D7: Error handling

Use `anyhow` for application-level errors and `thiserror` for typed error variants in the client layer. HTTP errors from Radarr include a JSON body with details — these should be parsed and displayed.

**Rationale:** `anyhow` provides clean error chains for CLI apps. `thiserror` gives structured errors where pattern matching is needed (e.g., distinguishing 404 from 500).

## Risks / Trade-offs

- **OpenAPI spec quality**: The Radarr spec may have inaccuracies or missing fields. → Mitigation: Test against a real Radarr instance early. Use `serde(deny_unknown_fields)` selectively to catch drift.

- **Progenitor compatibility**: The spec might use OpenAPI features that progenitor doesn't handle. → Mitigation: If the macro approach fails, use CLI codegen and patch the output. As a last resort, generate a partial client and hand-write problem endpoints.

- **Cleanup deletes data**: The disk cleanup feature permanently removes movies and their files. → Mitigation: Dry-run is the default mode. Actual deletion requires explicit `--confirm` flag. Output clearly shows what will be deleted before proceeding.

- **API rate limiting / large libraries**: Fetching history for every movie in a large library could be slow. → Mitigation: Use concurrent requests with a configurable concurrency limit. Cache movie data where possible.

- **137 schemas**: The generated code will be large. → Mitigation: Put generated code in its own module; use `#[allow(dead_code)]` on the generated module since not all types are used by every command.

## Open Questions

- Should we support multiple Radarr instances (profiles) in the config file? (e.g., `[profiles.home]`, `[profiles.seedbox]`)
- Should cleanup support excluding movies by tag or quality profile?
- Should we generate shell completions as part of the build or as a subcommand (`radarrer completions bash`)?
