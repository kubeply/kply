# AGENTS.md

## Purpose

`kply` is a Rust CLI that gives AI coding agents safe Kubernetes sessions
instead of raw production cluster access.

The project starts with a single binary and a multi-crate workspace. Keep the
normal `kply` path lightweight: no shell, Python, Node, or external
Kubernetes wrapper dependency in core session logic.

## Working Rules

- Read `CONTRIBUTING.md`, `crates/README.md`, and `docs/architecture.md` before
  making structural changes.
- Keep workspace crates focused. If a change touches multiple crates, the
  dependency direction should stay CLI -> config/checks/routing/k8s -> core,
  not the reverse.
- Placeholder crates are intentional. Put new implementation in the matching
  crate instead of expanding `kply-cli` by default.
- Keep session modeling, Kubernetes access, routing adapters, checks, config,
  and output separated.
- Prefer deterministic, auditable behavior over broad automation.
- Always attempt to add a test case for changed behavior.
- Prefer integration tests under each crate's `tests/` directory for CLI and
  workflow behavior. Use unit tests for model edge cases that are awkward to
  express through the CLI.
- Prefer `insta` snapshots for structured JSON output and reports instead of
  broad substring assertions.
- Avoid `panic!`, `unreachable!`, `.unwrap()`, `unsafe`, and clippy ignores in
  production code. In tests, use them sparingly when the failure would be
  clearer than manual error plumbing.
- Prefer fallible control flow such as `if let`, `let ... else`, and `Result`
  propagation over assuming success.
- Prefer let chains over nested `if let` statements when they make the code
  easier to read.
- If `unsafe` is ever required, include a `SAFETY:` comment explaining the
  invariant being upheld.
- Prefer `#[expect(...)]` over `#[allow(...)]` when a lint must be suppressed,
  and keep the reason local and specific.
- Never assume clippy warnings are pre-existing; keep `main` warning-free.
- Prefer top-level imports over local imports or fully qualified names when a
  type or function is used more than once.
- Avoid shortened variable names. Use names like `session`, `workload`, and
  `routing_adapter` instead of abbreviations.
- Prefer [`TypeName`] references in Rust doc comments for public APIs.
- Never run release builds unless the task is explicitly about release
  packaging or performance measurement.
- Prefer running a specific test during iteration, then run the full validation
  set before finishing.
- Never update the whole lockfile casually. If a dependency must change, keep
  the lockfile diff scoped and prefer `cargo update --package <name> --precise
  <version>`.

## Workspace Crate Inventory

- `kply-checks`: runtime verification checks and report generation.
- `kply-cli`: command parsing and user/agent-facing output.
- `kply-config`: project and cluster config parsing.
- `kply-core`: domain model, session state, audit events.
- `kply-k8s`: Kubernetes discovery and mutation adapters.
- `kply-routing`: Gateway, ingress, mesh, and fallback routing adapters.
- `kply-test`: shared test helpers.
- `xtask`: repository automation tasks.

## Validation

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo xtask check-crate-inventory-docs
cargo xtask check-module-docs
cargo xtask check-placeholder-docs
cargo xtask check-placeholders
```
