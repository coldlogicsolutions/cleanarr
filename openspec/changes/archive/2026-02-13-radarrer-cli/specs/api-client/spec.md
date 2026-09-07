## ADDED Requirements

### Requirement: Generated HTTP client from OpenAPI spec
The system SHALL generate a typed Rust HTTP client from the Radarr v3 OpenAPI specification (`openapi.json`) using progenitor. The generated client SHALL cover all 237 API endpoints with strongly-typed request and response models derived from the 137 API schemas.

#### Scenario: Client generation at build time
- **WHEN** the project is built with `cargo build`
- **THEN** the build process SHALL generate a Rust client module from `openapi.json` with typed methods for every API endpoint

#### Scenario: Type safety for API calls
- **WHEN** a CLI command invokes an API endpoint
- **THEN** the request parameters and response body SHALL be represented as typed Rust structs, not raw JSON

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
- **WHEN** `--url` and `--api-key` flags are provided
- **THEN** those values SHALL be used regardless of env vars or config file settings

#### Scenario: Environment variables override config file
- **WHEN** `RADARR_URL` or `RADARR_API_KEY` env vars are set and no CLI flags are provided
- **THEN** the env var values SHALL be used, overriding config file values

#### Scenario: Config file provides defaults
- **WHEN** no CLI flags or env vars are set
- **THEN** the system SHALL read connection settings from `~/.config/radarrer/config.toml`

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
