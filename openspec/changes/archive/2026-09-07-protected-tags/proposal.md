## Why

`--exclude-tag` must be repeated on every `cleanup` invocation, so a scripted or scheduled cleanup can silently delete a movie or show the user meant to keep. A persistent list of protected tags makes "never delete this" a one-time configuration.

## What Changes

- New `protected_tags` array under the `[cleanup]` section of the config file (e.g. `protected_tags = ["keep", "kids"]`).
- New `RADARR_PROTECTED_TAGS` env var (comma-separated) with the usual CLI > env > file precedence, where `--exclude-tag` is the CLI layer and is unioned with (not replacing) the configured list.
- `radarrer init --protected-tags keep,kids` persists the list.
- Protected tag names apply to Radarr movie tags and Sonarr series tags; an episode is protected when its series carries a protected tag.
- Dry-run/table output shows which protected tag names are in effect.

## Capabilities

### New Capabilities
- (none)

### Modified Capabilities
- `disk-cleanup`: "Cleanup respects movie exclusions" gains the configured protected list and its precedence rules.
- `cli-interface`: config file `[cleanup] protected_tags`, `RADARR_PROTECTED_TAGS`, and `init --protected-tags`.

## Impact

- `src/config.rs`: `CleanupConfig.protected_tags`, `AppConfig.protected_tags`, `ConfigUpdates.protected_tags`.
- `src/main.rs`: `--protected-tags` on `init`; pass resolved list to cleanup.
- `src/commands/cleanup.rs`: union with `--exclude-tag`; output line.
