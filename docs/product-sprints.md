# Product Sprints

Kply now plans product work through small OpenSpec-backed sprints instead of a
large milestone roadmap.

The foundation roadmap is archived at
[implementation-roadmap-v0-foundation.md](archive/implementation-roadmap-v0-foundation.md).
It remains useful as history, not as the active execution plan.

## Active Sprint

- OpenSpec change: [cluster-init-sprint](../openspec/changes/cluster-init-sprint/proposal.md)
- Goal: make Kply useful on a real Kubernetes cluster before asking users or
  agents to write config by hand.
- First product moment: `kply init --from-cluster` generates a starter
  `kply.yaml` from read-only Kubernetes discovery.

## Sprint Rules

1. Keep each sprint small enough to review and ship independently.
2. Start from a real user command, not internal architecture.
3. Keep human output readable and visual while preserving stable JSON for
   agents.
4. Avoid cluster mutation unless the sprint explicitly requires an `--apply`
   path.
5. Update or close the OpenSpec change when the sprint lands.
