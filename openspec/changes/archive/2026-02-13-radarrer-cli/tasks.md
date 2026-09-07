## 1. Project Scaffolding

- [x] 1.1 Initialize Rust binary crate with `cargo init --name radarrer`
- [x] 1.2 Add dependencies to `Cargo.toml`: clap (derive), reqwest (rustls-tls), serde, serde_json, tokio, anyhow, thiserror, comfy-table, toml, directories, progenitor-client
- [x] 1.3 Create module structure: `src/{config.rs, client.rs, output.rs, commands/mod.rs}`
- [x] 1.4 Set up `build.rs` with progenitor-macro for OpenAPI client generation from `openapi.json`

## 2. API Client Generation

- [x] 2.1 Configure progenitor to generate typed client from `openapi.json` in `build.rs`
- [x] 2.2 Create `src/client.rs` wrapper that initializes the generated client with base URL and API key header
- [x] 2.3 Verify generated client compiles and covers the major endpoint groups (movie, queue, history, system)
- [x] 2.4 Handle any OpenAPI spec issues — patch spec or add workarounds for progenitor limitations

## 3. Configuration

- [x] 3.1 Implement config file parsing from `~/.config/radarrer/config.toml` using toml + directories crates
- [x] 3.2 Implement environment variable reading (`RADARR_URL`, `RADARR_API_KEY`)
- [x] 3.3 Implement config layering: CLI flags > env vars > config file with proper merge logic
- [x] 3.4 Add config validation and clear error messages for missing required fields (url, api_key)

## 4. Output Formatting

- [x] 4.1 Define `OutputFormat` enum (Table, Json, Quiet) and global output selection via `--output` flag
- [x] 4.2 Implement table formatting helper using comfy-table with terminal-width-aware column layout
- [x] 4.3 Implement JSON output mode (pretty-printed serde_json)
- [x] 4.4 Implement quiet output mode (IDs/counts only, no headers)

## 5. CLI Structure & Global Flags

- [x] 5.1 Define top-level `Cli` struct with clap derive: global flags (`--url`, `--api-key`, `--output`, `--config`)
- [x] 5.2 Define `Commands` enum with subcommands for each resource group
- [x] 5.3 Wire up `main.rs`: parse args, load config, initialize client, dispatch to command handlers
- [x] 5.4 Implement `completions` subcommand for shell completion generation (bash, zsh, fish, powershell)

## 6. Core Command Modules — Movie

- [x] 6.1 Implement `movie list` — `GET /api/v3/movie` with table/json/quiet output
- [x] 6.2 Implement `movie get <id>` — `GET /api/v3/movie/{id}`
- [x] 6.3 Implement `movie add` — `POST /api/v3/movie` with required fields as flags
- [x] 6.4 Implement `movie update <id>` — `PUT /api/v3/movie/{id}`
- [x] 6.5 Implement `movie delete <id>` — `DELETE /api/v3/movie/{id}` with `--delete-files` flag
- [x] 6.6 Implement `movie lookup` — `GET /api/v3/movie/lookup` with `--term`, `--imdb`, `--tmdb` variants
- [x] 6.7 Implement `movie edit` (bulk) — `PUT /api/v3/movie/editor` with `--ids` and field flags
- [x] 6.8 Implement `movie import` — `POST /api/v3/movie/import`

## 7. Core Command Modules — Queue & History

- [x] 7.1 Implement `queue list` — `GET /api/v3/queue` with pagination flags
- [x] 7.2 Implement `queue details` — `GET /api/v3/queue/details`
- [x] 7.3 Implement `queue status` — `GET /api/v3/queue/status`
- [x] 7.4 Implement `queue delete <id>` and `queue delete --ids` (bulk) — `DELETE /api/v3/queue/{id}` and `/bulk`
- [x] 7.5 Implement `queue grab <id>` and `queue grab --ids` — `POST /api/v3/queue/grab/{id}` and `/bulk`
- [x] 7.6 Implement `history list` — `GET /api/v3/history` with pagination and sort flags
- [x] 7.7 Implement `history movie <id>` — `GET /api/v3/history/movie`
- [x] 7.8 Implement `history since` — `GET /api/v3/history/since`
- [x] 7.9 Implement `history failed <id>` — `POST /api/v3/history/failed/{id}`

## 8. Core Command Modules — System

- [x] 8.1 Implement `system health` — `GET /api/v3/health`
- [x] 8.2 Implement `system diskspace` — `GET /api/v3/diskspace`
- [x] 8.3 Implement `system status` — `GET /api/v3/system/status`
- [x] 8.4 Implement `system backup list` — `GET /api/v3/system/backup`
- [x] 8.5 Implement `system backup delete <id>` — `DELETE /api/v3/system/backup/{id}`
- [x] 8.6 Implement `system backup restore <id>` — `POST /api/v3/system/backup/restore/{id}`
- [x] 8.7 Implement `system task list` and `system task get <id>` — `GET /api/v3/system/task`
- [x] 8.8 Implement `system log list` with pagination — `GET /api/v3/log`
- [x] 8.9 Implement `system log files` and `system log file <filename>` — log file endpoints

## 9. Command Modules — Collections & Calendar

- [x] 9.1 Implement `collection list` — `GET /api/v3/collection`
- [x] 9.2 Implement `collection get <id>` — `GET /api/v3/collection/{id}`
- [x] 9.3 Implement `collection update <id>` — `PUT /api/v3/collection/{id}`
- [x] 9.4 Implement `calendar list` — `GET /api/v3/calendar` with `--start` and `--end` date flags

## 10. Command Modules — Configuration

- [x] 10.1 Implement `config host get` / `config host update` — host config endpoints
- [x] 10.2 Implement `config ui get` / `config ui update` — UI config endpoints
- [x] 10.3 Implement `config naming get` / `config naming update` / `config naming examples` — naming config
- [x] 10.4 Implement `config mediamanagement get` / `config mediamanagement update`
- [x] 10.5 Implement `config downloadclient get` / `config downloadclient update`
- [x] 10.6 Implement `config indexer get` / `config indexer update`
- [x] 10.7 Implement `config importlist get` / `config importlist update`
- [x] 10.8 Implement `config metadata get` / `config metadata update`

## 11. Command Modules — Profiles & Quality

- [x] 11.1 Implement `qualityprofile list/get/create/update/delete` — full CRUD
- [x] 11.2 Implement `qualitydefinition list/get/update` — quality definition endpoints
- [x] 11.3 Implement `customformat list/get/create/update/delete` — custom format CRUD plus bulk ops
- [x] 11.4 Implement `delayprofile list/get/create/update/delete/reorder` — delay profile CRUD
- [x] 11.5 Implement `releaseprofile list/get/create/update/delete` — release profile CRUD

## 12. Command Modules — Providers (Indexer, Download Client, Notification, Import List)

- [x] 12.1 Implement `indexer list/get/create/update/delete` plus `test`, `testall`, `schema`, and bulk ops
- [x] 12.2 Implement `downloadclient list/get/create/update/delete` plus `test`, `testall`, `schema`, and bulk ops
- [x] 12.3 Implement `notification list/get/create/update/delete` plus `test`, `testall`, `schema`
- [x] 12.4 Implement `importlist list/get/create/update/delete` plus `test`, `testall`, `schema`, bulk, and movie import
- [x] 12.5 Implement `metadata list/get/create/update/delete` plus `test`, `testall`, `schema`

## 13. Command Modules — Tags, Blocklist, Exclusions

- [x] 13.1 Implement `tag list/get/create/update/delete` plus `tag detail` — full tag CRUD
- [x] 13.2 Implement `blocklist list` with pagination, `blocklist movie`, `blocklist delete`, `blocklist delete --ids` (bulk)
- [x] 13.3 Implement `exclusions list/get/create/update/delete` plus `exclusions paged` and bulk ops

## 14. Command Modules — Remaining Resources

- [x] 14.1 Implement `moviefile list/get/update/delete` plus bulk operations
- [x] 14.2 Implement `release search` and `release push` — release endpoints
- [x] 14.3 Implement `command list/get/run/cancel` — command execution endpoints
- [x] 14.4 Implement `parse` — `GET /api/v3/parse` with `--title` flag
- [x] 14.5 Implement `rename` — `GET /api/v3/rename` with `--movie-id` flag
- [x] 14.6 Implement `manualimport list/create` — manual import endpoints
- [x] 14.7 Implement `remotepathmapping list/get/create/update/delete`
- [x] 14.8 Implement `credit list/get`, `alttitle list/get`, `extrafile list`
- [x] 14.9 Implement `autotagging list/get/create/update/delete` plus schema
- [x] 14.10 Implement `customfilter list/get/create/update/delete`
- [x] 14.11 Implement `language list/get`, `indexerflag list`, `filesystem browse/type/mediafiles`

## 15. Disk Cleanup Feature

- [x] 15.1 Implement disk usage check: query `GET /api/v3/diskspace`, parse sizes, compare against `--max-size` threshold
- [x] 15.2 Implement size parsing for human-readable units (B, KB, MB, GB, TB)
- [x] 15.3 Implement `oldest-added` strategy: fetch all movies, sort by `added` date ascending
- [x] 15.4 Implement `least-activity` strategy: fetch history per movie with concurrent requests, sort by event count ascending
- [x] 15.5 Implement `oldest-activity` strategy: fetch history per movie, sort by most recent event date ascending
- [x] 15.6 Implement candidate selection: iterate sorted list, accumulate sizes until enough space would be freed
- [x] 15.7 Implement dry-run output: table of candidates with Title, Size, Added Date, Last Activity, total to free
- [x] 15.8 Implement confirmed deletion: delete movies via API with progress reporting (`[3/10] Deleted "Title" (4.2 GB freed)`)
- [x] 15.9 Implement `--exclude-tag` flag to skip movies with specified tags
- [x] 15.10 Implement `--concurrency` flag for controlling parallel history fetches (default 5)

## 16. Error Handling & Polish

- [x] 16.1 Implement structured error types with thiserror for client errors (connection, auth, API errors)
- [x] 16.2 Parse Radarr JSON error responses and display human-readable messages
- [x] 16.3 Add `--verbose` flag for debug output (request URLs, response status codes)
- [x] 16.4 Test against a real Radarr instance for the most critical paths (movie list, system health, cleanup dry-run)
