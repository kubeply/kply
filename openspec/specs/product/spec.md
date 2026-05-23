# product Specification

## Purpose

Define the first open-source Kubeply CLI product boundary.

## Requirements

### Requirement: Kubeply exposes safe sessions to agents

Kubeply SHALL expose a CLI that AI coding agents and humans can use to create
safe Kubernetes-oriented sessions. The CLI SHALL prefer constrained commands and
auditable output over raw cluster mutation.

#### Scenario: Agent creates a dry-run session

- **WHEN** an agent creates a session with a workload, namespace, proposed image,
  and route header
- **THEN** Kubeply returns a deterministic session plan
- **AND** the plan identifies the workload, namespace, sandbox image, route
  header, initial checks, and cleanup expectation
- **AND** no Kubernetes resource is mutated when `--dry-run` is used

### Requirement: Kubeply remains a boundary, not a CD platform

Kubeply SHALL integrate with existing Kubernetes, CI/CD, GitOps, and routing
systems rather than replacing them as the first product wedge.

#### Scenario: A deployment workflow is proposed

- **WHEN** a workflow requires full release orchestration
- **THEN** Kubeply treats promotion as an integration point
- **AND** keeps the product surface focused on agent sessions, verification,
  route isolation, and audit trails
