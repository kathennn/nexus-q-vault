# Product specification — MVP 0.1

## Target user
A person with no command-line or cryptography experience who wants to protect documents, photos and personal backups on a computer they control.

## Single MVP use case
A **local encrypted file vault**. No network account or cloud service is required.

## Critical user journeys
1. **Create a vault:** choose a location, create a strong password, create and verify a recovery method, then receive clear confirmation of what is saved.
2. **Protect and restore files:** add files or folders, show progress, preserve originals unless the user explicitly chooses otherwise, and restore files to a chosen destination.
3. **Recover access:** use the documented recovery mechanism or a separately encrypted backup, with clear warnings about what can and cannot be recovered.

## MVP requirements
- Create, open, lock and unlock a versioned local vault.
- Encrypt and decrypt individual files through nexusq-core.
- Protect folder contents through a versioned encrypted manifest (not implemented).
- Create, verify and restore an encrypted backup.
- Detect corruption and fail closed: never return partial plaintext.
- Make export/migration possible without a proprietary service.
- Telemetry disabled by default; never collect content, filenames, paths, keys or recovery data.

## Recovery design constraints
- Recovery must be designed and tested before broad release.
- Never equate a backup passphrase with the vault password.
- Recovery phrases must encode sufficient entropy and have a versioned, checksummed, human-verifiable representation.
- The phrase must not be logged, transmitted or stored by the app in plaintext.
- The precise recovery mechanism remains an open design decision; do not promise password reset or phrase-based recovery until core support and tests exist.

## Logical data model
- Vault: format version, encrypted payload, KDF parameters and minimal public header fields.
- Protected file: opaque identifier, encrypted payload and integrity data.
- Manifest: encrypted mapping of opaque identifiers to file metadata and content references.
- Activity: local-only privacy-minimized events; never include file contents, names, absolute paths, secrets or recovery material.

## Current prototype limitations

The desktop prototype currently encrypts individual files to sidecar `.nqx` envelopes and stores key records in a separate `.nqv` vault. It does not yet provide an encrypted folder/manifest. Its `.nqb` backup contains vault/key state, not the `.nqx` file payloads; users must preserve both separately. The backup-based recovery journey is a proposal in [docs/RECOVERY_DESIGN.md](docs/RECOVERY_DESIGN.md), not yet user-tested or approved for beta.

## Explicitly out of scope
Synchronization, cloud storage, sharing links, collaboration, teams, enterprise policy, remote administration and multi-device identity.

## Acceptance criteria
- A new user can create and unlock a vault without a terminal.
- Wrong passwords and modified ciphertext fail safely.
- Files round-trip byte-for-byte.
- A backup restores into a new vault with separately supplied credentials.
- Recovery instructions are understandable and validated by usability testing.
- Format and migration behavior are documented and tested.
