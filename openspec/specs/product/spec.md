# product Specification

## Purpose

Define the first open-source Kubeply CLI product boundary through the `kply`
tool.

## Requirements

### Requirement: Kply defines safe sessions for agents

Kply SHALL define a CLI architecture that AI coding agents and humans can use
to create safe Kubernetes-oriented sessions once the roadmap is implemented.
The CLI SHALL prefer constrained commands and auditable output over raw cluster
mutation.

#### Scenario: Placeholder CLI is run

- **WHEN** an agent or human runs the placeholder CLI
- **THEN** Kply returns deterministic placeholder output
- **AND** no Kubernetes resource is mutated

### Requirement: Kply remains a boundary, not a CD platform

Kply SHALL integrate with existing Kubernetes, CI/CD, GitOps, and routing
systems rather than replacing them as the first product wedge.

#### Scenario: A deployment workflow is proposed

- **WHEN** a workflow requires full release orchestration
- **THEN** Kply treats promotion as an integration point
- **AND** keeps the product surface focused on agent sessions, verification,
  route isolation, and audit trails
