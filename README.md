# cleanarr

Keep a shared media disk under a size budget. `cleanarr` looks at what Radarr
and Sonarr have on disk, compares the total against a threshold you set, and
deletes the least valuable movies and TV episodes until you're back under it,
using Plex watch history to decide what "least valuable" means.

The repository is a Cargo workspace with three crates:

| Crate | Kind | Purpose |
|---|---|---|
| `cleanarr` (root) | binary | The cleanup tool described below |
| `crates/radarr-api` | library | Thin, hand-written client for the Radarr v3 HTTP API |
| `crates/radarr-cli` | binary | Complete command-line client for Radarr, built on `radarr-api` |

`radarr-api` and `radarr-cli` have no dependency on `cleanarr` and are laid out
so they can move to their own repository later.

## Features

- **One threshold for movies and TV.** Radarr movie size plus Sonarr series
  size is compared against `--max-size`; both kinds are deletion candidates.
- **Five strategies.** `oldest` (last watched, else added), `oldest-added`,
  `never-watched`, `least-watched`, `oldest-watched`.
- **Plex-aware.** Movies match by TMDB/IMDB id; episodes match by show id plus
  season and episode number. Works without Plex too, falling back to added date.
- **Safe by default.** Dry run unless `--confirm`. Protected tags can never be
  deleted. Deleted episodes are unmonitored so Sonarr won't re-grab them.
- **Scriptable.** `--output json` and `--output quiet`, non-zero exit on any
  failed deletion, defaults readable from config or environment.

## Install

```sh
cargo install --git https://github.com/derekjw/cleanarr cleanarr
cargo install --git https://github.com/derekjw/cleanarr radarr-cli   # optional
```

Or with [devenv](https://devenv.sh): `devenv shell` gives you the toolchain,
then `cargo build --release`.

## Setup

```sh
cleanarr init --radarr-url http://localhost:7878 --radarr-api-key RADARR_KEY
cleanarr init --sonarr-url http://localhost:8989 --sonarr-api-key SONARR_KEY
cleanarr init --plex-url http://localhost:32400
cleanarr plex login          # browser-based, stores a Plex token
cleanarr init --protected-tags keep,kids --default-max-size 4TB
```

`init` merges into `~/.config/cleanarr/config.toml`:

```toml
[radarr]
url = "http://localhost:7878"
api_key = "..."

[sonarr]
url = "http://localhost:8989"
api_key = "..."

[plex]
url = "http://localhost:32400"
token = "..."

[cleanup]
default_max_size = "4TB"
default_strategy = "oldest"
protected_tags = ["keep", "kids"]
```

Every setting can also come from a flag or an environment variable, with
precedence CLI > env > file:

| Setting | Flag | Env |
|---|---|---|
| Radarr | `--radarr-url`, `--radarr-api-key` | `RADARR_URL`, `RADARR_API_KEY` |
| Sonarr | `--sonarr-url`, `--sonarr-api-key` | `SONARR_URL`, `SONARR_API_KEY` |
| Plex | `--plex-url`, `--plex-token` | `PLEX_URL`, `PLEX_TOKEN` |
| Output format | `--output` | `CLEANARR_OUTPUT` |
| Threshold | positional `MAX_SIZE` | `CLEANARR_CLEANUP_MAX_SIZE` |
| Strategy | `--strategy` | `CLEANARR_CLEANUP_STRATEGY` |
| Protected tags | `--exclude-tag` (adds to the list) | `CLEANARR_PROTECTED_TAGS` |

Sonarr and Plex are optional. Without Sonarr only movies are counted and
deleted. Without Plex the watch-based strategies are unavailable.

## Cleanup

```sh
# See what would go (dry run)
cleanarr cleanup 4TB --strategy never-watched

# Do it
cleanarr cleanup 4TB --strategy never-watched --confirm

# Only touch TV, keep everything tagged "keep", use configured threshold
cleanarr cleanup --media tv --exclude-tag keep --confirm
```

Example dry run:

```
Disk /data: 4.6 TB used / 5.0 TB total (400.0 GB free)
Library: 4.3 TB (movies 1.8 TB, TV 2.5 TB) (threshold: 4.0 TB). Need to free: 300.0 GB

Protected tags: keep

┌─────────┬──────┬────────────────────────────────────┬─────────┬────────────┬───────┬──────────────┐
│ Type    │ ID   │ Title                              │ Size    │ Added      │ Views │ Last Watched │
├─────────┼──────┼────────────────────────────────────┼─────────┼────────────┼───────┼──────────────┤
│ episode │ 4821 │ Some Show - S01E01 - Pilot         │ 3.1 GB  │ 2023-02-11 │ 0     │ -            │
│ movie   │ 212  │ A Film                             │ 8.4 GB  │ 2023-03-02 │ 0     │ -            │
...
Total: 12 movies, 41 episodes, 301.2 GB to free

Dry run -- nothing deleted. Use --confirm to delete.
```

Options:

| Flag | Meaning |
|---|---|
| `MAX_SIZE` | Threshold, e.g. `500GB`, `4TB` (SI units). Falls back to config/env. |
| `--strategy` | `oldest` (default), `oldest-added`, `never-watched`, `least-watched`, `oldest-watched` |
| `--media` | `all` (default), `movies`, `tv`. Disk usage always counts both. |
| `--exclude-tag` | Extra protected tag names, comma-separated |
| `--plex-section`, `--plex-tv-section` | Plex library keys; auto-detected when there is exactly one of each type |
| `--confirm` | Actually delete |

Movies are deleted through Radarr with their files. Episodes have their file
deleted through Sonarr and are then unmonitored. If any single deletion fails
the run continues, reports the item, and exits non-zero at the end.

## radarr-cli

A separate binary exposing every Radarr v3 API resource group as a subcommand:

```sh
radarr-cli init --url http://localhost:7878 --api-key RADARR_KEY
radarr-cli movie list
radarr-cli movie lookup --term "The Matrix"
radarr-cli queue list
radarr-cli system health
radarr-cli history list --page-size 50 --output json
```

Settings: `--url` / `--api-key` / `--output` (`--radarr-url` / `--radarr-api-key` also accepted), env `RADARR_URL` / `RADARR_API_KEY` /
`RADARR_OUTPUT`, or `~/.config/radarr-cli/config.toml`. Run `radarr-cli --help`
for the full list and `radarr-cli <command> --help` for each one.
`radarr-cli completions <shell>` prints shell completions.

## radarr-api

```rust
use radarr_api::{RadarrClient, types::MovieResource};

let api = RadarrClient::new("http://localhost:7878", "KEY")?;
let movies: Vec<MovieResource> = api.get("/api/v3/movie").await?;
```

The client handles the API key header, JSON encoding, and error reporting.
Callers choose the endpoint path and the response type.

## Development

The project uses [OpenSpec](https://github.com/Fission-AI/OpenSpec) for
spec-driven development; behaviour specs live in `openspec/specs/` and the
history of changes in `openspec/changes/archive/`.

```sh
devenv shell
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
openspec validate --specs
```

Entering the devenv shell installs git pre-commit hooks that run rustfmt,
clippy (warnings are errors), and `openspec validate --specs`, so commits fail
if the code is unformatted, has lints, or the specs are malformed. Run them by
hand with `prek run --all-files`.

## License

MIT. See [LICENSE](LICENSE).
