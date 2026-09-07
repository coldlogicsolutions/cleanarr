## ADDED Requirements

### Requirement: Cleanup defaults configuration
The system SHALL persist cleanup defaults via `radarrer init --default-max-size <SIZE>` and `--default-strategy <STRATEGY>` under `[cleanup]`, and SHALL honour `RADARR_CLEANUP_MAX_SIZE` and `RADARR_CLEANUP_STRATEGY` environment variables.

#### Scenario: Init saves defaults
- **WHEN** the user runs `radarrer init --default-max-size 4TB --default-strategy oldest-added`
- **THEN** the config file SHALL contain both values under `[cleanup]`
