# Local Demo Terminal Cast

This page ships a lightweight terminal cast for the local Kind demo:

- [demo-terminal-cast.cast](demo-terminal-cast.cast)

The cast is an asciinema v2 text recording. It is intentionally short and
illustrative: it shows the safe command sequence and current boundaries without
requiring a video file in the repository.

Replay it with asciinema when available:

```bash
asciinema play docs/demo-terminal-cast.cast
```

Or inspect it directly as newline-delimited JSON:

```bash
sed -n '1,20p' docs/demo-terminal-cast.cast
```

## What It Shows

The cast walks through the bounded local flow:

1. Check local demo prerequisites with `kply demo doctor`.
2. Create and select a disposable `kind-kply-demo` context.
3. Install the ecommerce fixture with `kply demo install`.
4. Inspect the configured `checkout` app.
5. Produce a dry-run session plan for a candidate checkout image.
6. Plan routing with the current preview-service fallback.
7. Apply only the documented fixed backend demo manifest.
8. Run a check report against the explicit `kply-demo` namespace.
9. Reset and tear down the demo resources.

## Boundaries

The cast does not claim production safety, route promotion, or automated
rollback. It demonstrates the current open-source CLI workflow: local
readiness, explicit config, read-only app inspection, dry-run planning, current
check output, and cleanup in a disposable Kind namespace.

Use [Local Kind Demo](demo-kind.md) for the runnable steps and
[Coding Agent Demo Guide](demo-agent.md) for the agent prompt.
