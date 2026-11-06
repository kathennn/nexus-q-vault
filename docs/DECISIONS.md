# Architecture decisions

## ADR-001 — One cryptographic engine
**Status:** accepted in principle; integration spike only.

NEXUS-Q Vault delegates cryptographic operations, vault container handling, key lifecycle and backup encryption to `nexusq-core`. The app must not add a second crypto provider or custom file encryption format. A narrow Rust application layer translates user intent into core API calls.

The spike uses a sibling source checkout via Cargo path dependency. Before public release, dependency provenance and version pinning must be formalized (immutable revision or reviewed published release, lockfile and supply-chain evidence).

## ADR-002 — Local-first MVP
**Status:** accepted. No account, cloud, synchronization, sharing or network service is required for the MVP.

## ADR-003 — Preserve source files
**Status:** accepted for MVP. Encryption never deletes the original automatically. Secure deletion cannot be guaranteed on modern filesystems and SSDs. Any future cleanup action must explain this limitation and require confirmation.

## ADR-004 — MIT license
**Status:** provisional project choice. MIT is simple and permissive. Revisit before first public release if contributors prefer stronger copyleft. License selection does not change licenses or notices required by dependencies.
