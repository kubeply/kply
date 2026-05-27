# Config Fixtures

Kply configuration fixtures follow `config/<case-name>/kply.yaml`.

Current valid fixtures:

- `minimal-defaults`: explicit schema version with defaulted sections.
- `complete-single-app`: one fully populated app using header routing.
- `multi-app-route-strategies`: multiple apps covering every route strategy.
- `policy-baseline`: one named policy entry with explicit status and context.

Current invalid fixtures:

- `invalid-empty-app-fields`: parseable config with required app fields blank.
- `invalid-unsupported-version`: parseable config with an unsupported schema version.
- `invalid-unknown-top-level-field`: config rejected during schema loading.
- `invalid-unknown-routing-field`: routing config rejected during schema loading.
