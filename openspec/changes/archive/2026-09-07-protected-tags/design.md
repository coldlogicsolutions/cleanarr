## Context

`cleanup` already resolves `--exclude-tag` labels against Radarr and Sonarr tag tables and skips tagged movies/series. The config file has a `[cleanup]` section (`default_strategy`, `default_max_size`) that is parsed but not currently consumed by the command.

## Goals / Non-Goals

**Goals:** persistent protection list; additive with the CLI flag; zero behaviour change when unset.
**Non-Goals:** per-title protection lists, protection by other metadata (quality, collection), or changing how unknown tag labels are handled (still ignored).

## Decisions

1. **Union, not override.** `--exclude-tag` normally sits at the top of CLI > env > file, but replacing a protection list from the command line would be a foot-gun. The effective set is `file ∪ env ∪ CLI`. Alternative: strict override — rejected as unsafe for a deletion tool.
2. **Resolved in `AppConfig`.** `AppConfig.protected_tags: Vec<String>` holds `file ∪ env`; cleanup unions in `args.exclude_tag`. Keeps cleanup unaware of config sources, matching how Plex/Sonarr configs are passed.
3. **Env format** is comma-separated to match `--exclude-tag`'s value delimiter.
4. **Init flag** `--protected-tags a,b` replaces the stored list (not append), so the file stays declarative.

## Risks / Trade-offs

- [Tag label typo silently protects nothing] → Table output prints the effective protected list so a dry run reveals it.
