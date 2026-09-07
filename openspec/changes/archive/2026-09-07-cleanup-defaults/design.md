## Context
`CleanupConfig` fields are deserialized but unused. `CleanupArgs.max_size` is a required `String`; `strategy` has `default_value = "oldest"`.

## Decisions
1. **Resolve in `AppConfig`, finish in cleanup.** `AppConfig` merges env over file into `CleanupDefaults { max_size, strategy }`; cleanup applies CLI over that and the `oldest` fallback. Mirrors how Plex/Sonarr/protected tags flow.
2. **Strategy validation stays in cleanup** so a bad configured value produces the same error as a bad flag.
3. **`init` flags replace** the stored values (consistent with the other init flags).

## Risks / Trade-offs
- [Positional becomes optional → `radarrer cleanup` with nothing configured] → clear error naming all three ways to set it.
