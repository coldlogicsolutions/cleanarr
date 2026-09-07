## 1. Config

- [x] 1.1 Add `protected_tags: Vec<String>` to `CleanupConfig`; resolve `file ∪ env` into `AppConfig.protected_tags`
- [x] 1.2 Add `protected_tags` to `ConfigUpdates`/`save_to_config`; add `--protected-tags` to `init`

## 2. Cleanup

- [x] 2.1 Union `AppConfig.protected_tags` with `--exclude-tag` in cleanup; print effective list in table mode
- [x] 2.2 Unit test the union/dedupe helper

## 3. Verify

- [x] 3.1 `cargo test`, `cargo clippy`, `--help` shows new flag
