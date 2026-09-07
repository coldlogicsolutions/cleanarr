## Purpose
Authenticated HTTP client for the Radarr v3 API with typed JSON handling and consistent error reporting.

## Requirements

### Requirement: Thin hand-written HTTP client
The `radarr-api` crate SHALL provide a hand-written client (`RadarrClient`) over reqwest: callers choose the endpoint path and the response type (any `serde::Deserialize` type, including `serde_json::Value`), and the client handles authentication, JSON encoding, timeouts, and error reporting. Commonly shared resource types (e.g. `MovieResource`) live in `radarr_api::types`; command modules define their own projections otherwise. No code generation or bundled OpenAPI document is used; the upstream specification is available from the Radarr repository or a running instance when needed.

#### Scenario: Typed or raw responses
- **WHEN** a caller invokes `api.get::<T>(path)`
- **THEN** the response body SHALL be deserialized into `T`, which may be a typed struct or raw `serde_json::Value`

#### Scenario: Client hardening
- **WHEN** the client is constructed
- **THEN** it SHALL set a connect timeout and an overall request timeout and SHALL NOT follow HTTP redirects, so the API key header is never forwarded to another origin

#### Scenario: Error messages identify the request
- **WHEN** a request fails with a non-success status or an unparseable body
- **THEN** the error SHALL include the HTTP method and URL (and, for parse failures, the content type and the start of the body)

### Requirement: Authentication via API key
The system SHALL authenticate all API requests by including the Radarr API key as the `X-Api-Key` HTTP header on every request.

#### Scenario: API key provided via config
- **WHEN** an API key is configured (via flag, env var, or config file)
- **THEN** all HTTP requests to the Radarr instance SHALL include the header `X-Api-Key: <key>`

#### Scenario: Missing API key
- **WHEN** no API key is provided through any configuration source
- **THEN** the system SHALL exit with an error message indicating the API key is required

### Requirement: Connection configuration
The system SHALL support configuring the Radarr instance URL and API key through three sources with the following priority (highest to lowest): CLI flags, environment variables, config file.

#### Scenario: CLI flags override all
- **WHEN** `--radarr-url` and `--radarr-api-key` flags are provided
- **THEN** those values SHALL be used regardless of env vars or config file settings

#### Scenario: Environment variables override config file
- **WHEN** `RADARR_URL` or `RADARR_API_KEY` env vars are set and no CLI flags are provided
- **THEN** the env var values SHALL be used, overriding config file values

#### Scenario: Config file provides defaults
- **WHEN** no CLI flags or env vars are set
- **THEN** the system SHALL read connection settings from `$XDG_CONFIG_HOME/cleanarr/config.toml`, falling back to `~/.config/cleanarr/config.toml` when `XDG_CONFIG_HOME` is unset or not absolute (on every platform)

#### Scenario: Config file does not exist
- **WHEN** no config file exists and no flags or env vars are provided
- **THEN** the system SHALL exit with an error indicating connection details are required

### Requirement: HTTP error handling
The system SHALL parse Radarr API error responses and present them as readable error messages.

#### Scenario: API returns 4xx/5xx with JSON error body
- **WHEN** the Radarr API returns an HTTP error status with a JSON body containing error details
- **THEN** the system SHALL display the error message from the response body, not the raw HTTP status

#### Scenario: Connection refused
- **WHEN** the Radarr instance is unreachable
- **THEN** the system SHALL display a clear error message indicating the connection failed, including the URL that was attempted
