## ADDED Requirements

### Requirement: Cleanup defaults from configuration
The system SHALL resolve the size threshold and strategy as CLI argument, then `RADARR_CLEANUP_MAX_SIZE` / `RADARR_CLEANUP_STRATEGY` environment variables, then `default_max_size` / `default_strategy` in the `[cleanup]` config section. Strategy SHALL fall back to `oldest`; a missing threshold SHALL be an error.

#### Scenario: Threshold from config
- **WHEN** the config file has `default_max_size = "4TB"` and the user runs `radarrer cleanup`
- **THEN** the system SHALL use 4 TB as the threshold

#### Scenario: CLI overrides config
- **WHEN** the config file has `default_max_size = "4TB"` and the user runs `radarrer cleanup 2TB`
- **THEN** the system SHALL use 2 TB

#### Scenario: Strategy from config
- **WHEN** the config file has `default_strategy = "never-watched"` and no `--strategy` is given
- **THEN** the system SHALL use the `never-watched` strategy, validating it like a CLI value

#### Scenario: No threshold anywhere
- **WHEN** no threshold is given on the CLI, in the environment, or in the config file
- **THEN** the system SHALL exit with an error naming all three ways to provide one
