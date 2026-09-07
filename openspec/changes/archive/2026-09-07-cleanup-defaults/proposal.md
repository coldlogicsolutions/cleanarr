## Why

The config file already accepts `[cleanup] default_strategy` and `default_max_size`, but `cleanup` ignores them and requires the threshold on every invocation. Wiring them up makes a bare `radarrer cleanup` usable in cron jobs.

## What Changes

- `cleanup`'s `MAX_SIZE` positional becomes optional; `--strategy` loses its hard-coded default.
- Resolution order for both: CLI > env (`RADARR_CLEANUP_MAX_SIZE`, `RADARR_CLEANUP_STRATEGY`) > config file > built-in (`oldest` for strategy; no built-in for max size, error if unresolved).
- `radarrer init --default-max-size 4TB --default-strategy oldest-added` persists them.

## Capabilities

### New Capabilities
- (none)

### Modified Capabilities
- `disk-cleanup`: threshold and strategy may come from config/env.
- `cli-interface`: new init flags and env vars.

## Impact

`src/config.rs` (`AppConfig.cleanup_defaults`, `ConfigUpdates`), `src/main.rs` (init flags), `src/commands/cleanup.rs` (optional args, resolution).
