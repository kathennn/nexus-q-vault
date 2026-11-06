# Contributing

Thank you for considering a contribution to NEXUS-Q Vault.

## Before you start

- Read [PRODUCT.md](PRODUCT.md), [THREAT_MODEL.md](THREAT_MODEL.md) and [docs/DECISIONS.md](docs/DECISIONS.md).
- Search existing issues before opening a new one.
- For substantial changes, open an issue first to agree on scope.

## Development

- Rust 1.90 or newer for the desktop application (the library crate declares Rust 1.88).
- Keep the cryptographic boundary narrow: all cryptographic operations belong to the reviewed `nexusq-core` API.
- Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` before submitting.
- Add tests for failure cases, corruption and interrupted operations.
- Never commit credentials, real user files, recovery phrases or key material.

## Translations and localization

- Check the existing language strings and open localization issues before starting a new translation or changing translation keys.
- Keep user-visible text translatable; avoid embedding interface copy in business logic or duplicating language-specific conditionals.
- Preserve the meaning and tone of the English source text. Do not translate cryptographic, security or recovery terminology by guesswork.
- Include every required interface string, including dialogs, validation errors, accessibility labels, tooltips, notifications and dynamic states.
- Use the project's existing localization structure and conventions. Propose a new library or file format in an issue before introducing a dependency.
- Keep English as the fallback for missing translations. Do not silently remove or alter another language's strings.
- Have translations reviewed by a fluent speaker before marking a language complete. Identify the language/locale and summarize what was translated and what remains incomplete in the pull request.
- Test language switching and check for clipped text, layout issues and incorrect formatting (including dates, numbers and right-to-left layout where applicable).

## Pull requests

A pull request should explain the user impact, security impact, tests performed, format compatibility impact and any migration implications. Changes to key handling, file formats, recovery, backups, dependencies or update verification must be explicitly marked security-sensitive.

## Security review

Security-sensitive changes require approval from designated maintainers before merge. Until a security-maintainer group and independent reviewers are formally appointed, the project must not claim that such review has occurred. Do not disclose vulnerabilities in public issues; follow SECURITY.md.

## License

By submitting a contribution, you agree that it is offered under the project's MIT license. Include attribution and license notices for third-party material.
