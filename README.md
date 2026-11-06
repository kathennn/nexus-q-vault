# NEXUS-Q Vault

**A local-first desktop vault for files, notes and backups, built around the NEXUS-Q cryptographic engine.**

> **Status: early development / technical spike. Not production-ready.** Do not rely on this project as the only copy of important data.

NEXUS-Q Vault aims to let people without security expertise protect local files with a clear recovery path. Source code, vault-format decisions, threat model and release process are intended to remain public and auditable.

## What it aims to protect
- Confidentiality and integrity of files stored in an encrypted local vault.
- Recovery of vault data from a separately encrypted backup.
- Cryptographic operations delegated to the `nexusq-core` Rust library.

## What it does not currently promise
- Protection from malware, keyloggers, a compromised operating system or an attacker controlling the logged-in user session.
- Secure deletion of original files, swap, filesystem snapshots or SSD blocks.
- Cloud synchronization, sharing, collaboration or enterprise administration.
- An independently audited implementation, certified cryptography, or reproducible release binaries.

## MVP scope
The MVP is one use case: a **local encrypted file vault**. It covers vault creation, file encryption/decryption and backup restoration. Synchronization, cloud services, sharing links, teams and enterprise management are explicitly out of scope.

See [PRODUCT.md](PRODUCT.md), [THREAT_MODEL.md](THREAT_MODEL.md), [docs/DECISIONS.md](docs/DECISIONS.md) and the [backup-based recovery proposal](docs/RECOVERY_DESIGN.md).

## Cryptographic boundary
`nexusq-core` is the sole cryptographic engine. Vault must not introduce parallel cryptographic implementations, custom key derivation, custom encryption formats or silent fallback paths. The integration spike uses the public `Vault`, `Session`, and `storage::backup` APIs from the sibling `nexus-q` checkout. This local path is a development convenience; a published build must pin a reviewed release or immutable revision and verify its provenance.

The engine revision pinned by CI is `efe486603af266c379131526d8320a0584254ce4`. Its file helpers publish outputs via a same-directory temporary file and atomic no-clobber hard link; on Unix the file is created with mode `0600` and the containing directory is synced. This is create-only publication, not atomic replacement. Non-Unix permission and durability semantics still require platform-specific validation. The engine's current default provider is RustCrypto. Experimental native backends are not enabled by this project. Exact algorithms and security claims must be tied to the engine version and its published documentation; "post-quantum" does not mean every operation or threat is quantum-proof.

## Repository status
This repository contains project governance, a Rust integration spike and an early Tauri 2 desktop prototype (`apps/desktop`). The prototype currently supports vault create/open/lock, single-file `.nqx` encryption/decryption and encrypted vault-key backup export/import through `nexusq-core`. Backups do not bundle `.nqx` payload files; those must be preserved separately. It is not yet user-tested or production-ready. Native file pickers, recovery workflow, encrypted folder manifest, format migration policy, release pipeline and independent security review are not complete.

## Run the desktop prototype

The desktop app uses Tauri 2 and requires the platform webview/build prerequisites documented by Tauri. From this repository root, with the sibling `../nexus-q` checkout at the revision pinned by this project:

```sh
cargo run --manifest-path apps/desktop/Cargo.toml
```

The current prototype accepts filesystem paths manually. Native file/folder pickers and installer bundling are not enabled yet. The desktop crate currently requires Rust 1.90 or newer; the cryptographic engine's own MSRV remains governed by `nexusq-core`.

## Build the spike
Prerequisites: Rust 1.88+ and a sibling checkout of `../nexus-q` containing the matching `nexusq-core` workspace.

```sh
cargo test
```

The spike is a library integration test, not a finished user-facing application. Do not put real passwords in shell history or command-line arguments.

## Governance
- License: MIT. See [LICENSE](LICENSE).
- Contributions: [CONTRIBUTING.md](CONTRIBUTING.md).
- Security reports: [SECURITY.md](SECURITY.md).
- Community standards: [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
- MVP definition: [PRODUCT.md](PRODUCT.md).
- Threat model: [THREAT_MODEL.md](THREAT_MODEL.md).
- Decisions: [docs/DECISIONS.md](docs/DECISIONS.md).

## Current file-operation constraints

The pinned engine reads complete inputs and envelopes into memory; it does not provide streaming file encryption or bounded-memory processing. File outputs now use atomic create-only publication: a completed temporary file is linked into place without overwriting an existing destination. On Unix, file data and the parent directory are synced and new files use mode `0600`. This does not provide atomic replacement, secure deletion, or validated permission/durability semantics on every platform. Consequently, the current integration is a correctness spike for modest file sizes, not yet a safe general-purpose folder vault. Large-file support and cross-platform filesystem guarantees remain release gates. Authentication failures are tested to produce no plaintext output.

## Prepare the local engine checkout

The current spike uses a sibling path dependency, so clone the exact engine
revision used for the integration test:

```sh
git clone https://github.com/lecodev-26/nexus-q.git ../nexus-q
git -C ../nexus-q checkout efe486603af266c379131526d8320a0584254ce4
cargo test --locked
```

The CI workflow performs the same pinned checkout. Updating the engine revision
is a security-sensitive dependency change: review the engine release notes,
cryptography documentation, dependency diff and compatibility tests first.
