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

## ADR-005 — Do not paper over current file I/O limitations
**Status:** accepted for the integration phase; release gate remains open.

The pinned `nexusq-core` revision (`efe486603af266c379131526d8320a0584254ce4`) still loads complete inputs and envelopes into memory. Its file helpers now write to exclusive same-directory temporary files, sync file contents, and publish with a hard link that fails if the destination already exists. On Unix, the temporary file is created with mode `0600` and the parent directory is synced. This provides atomic create-only publication, not atomic replacement. Non-Unix permission and durability semantics need platform validation.

NEXUS-Q Vault will keep using the engine as its only cryptographic implementation. It will not introduce a parallel envelope format or home-grown streaming cipher to compensate. Before beta, the integration still needs bounded resource use, explicit platform guarantees, overwrite UX/policy, and broader filesystem fault-injection tests. The upstream implementation is in `nexus-q` commit `efe4866`; it is not an independent security audit.

The application-level tests now cover wrong vault passwords, successful byte-for-byte file and backup roundtrips, and tampered-envelope rejection without creating a plaintext output. These tests do not establish crash consistency, large-file safety, or independent security review.
