## 1. Implementation

- [x] 1.1 Add `CleanupDefaults` to `AppConfig` (env over file); extend `ConfigUpdates`/`init` with `--default-max-size`, `--default-strategy`
- [x] 1.2 Make `max_size` optional and `strategy` an `Option` in `CleanupArgs`; resolve with fallbacks and clear error
- [x] 1.3 Tests for resolution helper; `cargo test`/`clippy`
