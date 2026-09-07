## ADDED Requirements

### Requirement: Protected tags configuration
The system SHALL read a `protected_tags` string array from the `[cleanup]` section of the config file and a comma-separated `RADARR_PROTECTED_TAGS` environment variable, and SHALL persist the list via `radarrer init --protected-tags <a,b,...>`.

#### Scenario: Init saves protected tags
- **WHEN** the user runs `radarrer init --protected-tags keep,kids`
- **THEN** the config file SHALL contain `protected_tags = ["keep", "kids"]` under `[cleanup]`, replacing any previous list

#### Scenario: Env var parsed
- **WHEN** `RADARR_PROTECTED_TAGS=keep, kids` is set
- **THEN** the system SHALL treat "keep" and "kids" (trimmed, empty entries dropped) as protected
