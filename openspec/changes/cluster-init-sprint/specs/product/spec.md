## ADDED Requirements

### Requirement: Kply initializes config from a live cluster

Kply SHALL provide a read-only init workflow that discovers candidate
applications from the current Kubernetes context and writes a starter config
without requiring users or agents to hand-author `kply.yaml` first.

#### Scenario: User initializes from cluster

- **WHEN** a user runs `kply init --from-cluster`
- **THEN** Kply discovers candidate Deployments and matching Services
- **AND** writes a starter `kply.yaml`
- **AND** prints the discovered apps and next commands
- **AND** does not create, update, delete, or patch Kubernetes resources

#### Scenario: Existing config is protected

- **WHEN** `kply.yaml` already exists
- **AND** the user does not pass `--overwrite`
- **THEN** Kply refuses to replace the file
- **AND** explains which flag or output path to use

### Requirement: Kply separates human terminal UI from agent JSON

Kply SHALL make human output grouped, indented, and color-aware while keeping
JSON output deterministic and undecorated for agents.

#### Scenario: Human output is visual

- **WHEN** a user runs `kply init --from-cluster`
- **THEN** Kply groups cluster facts, discovered apps, generated files, and next
  commands into readable sections
- **AND** honors `--no-color`

#### Scenario: Agent output is stable

- **WHEN** an agent runs `kply init --from-cluster --json`
- **THEN** Kply returns a deterministic JSON object
- **AND** does not include ANSI color or human-only decoration
