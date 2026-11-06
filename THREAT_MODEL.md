# Threat model — NEXUS-Q Vault MVP

**Status:** initial, living document; review required before beta.
**Scope:** local desktop file vault, file encryption, recovery and backups.

## Assets

| Asset | Sensitivity | Examples |
|---|---|---|
| Plaintext files | Critical | Documents, photos, personal records |
| Vault and backup credentials | Critical | Passwords, recovery material |
| Derived keys and key records | Critical | In-memory secrets, wrapped key material |
| Encrypted vault and backup | High | Ciphertext, headers, KDF parameters |
| File manifest and metadata | High | Names, sizes, folder structure, timestamps |
| Update and release trust | High | Signing keys, build provenance |

## Adversaries
- A person who obtains a powered-off device or copies its disk.
- A local unprivileged user who can read files accessible to their account.
- Malware running with the user's privileges.
- A privileged or compromised OS/kernel.
- A malicious dependency, compromised build runner or release account.
- A future quantum-capable attacker targeting vulnerable public-key cryptography.

## Security goals
- Confidentiality and authenticated integrity of protected file contents at rest.
- Wrong passwords, malformed files and tampering must fail closed.
- Minimize plaintext and secret lifetime in memory and on disk.
- Make backup and restoration independently verifiable.
- Keep content, names, paths, keys and recovery data off telemetry and logs.
- Preserve format versioning and documented export paths.

## Trust boundaries
1. Desktop UI to Rust application service.
2. Application service to nexusq-core (the only cryptographic engine).
3. Process memory to local filesystem.
4. Main vault to separately encrypted backup.
5. Source repository/build pipeline to distributed installers.

Every boundary needs explicit input validation and error handling. UI validation is not a security boundary.

## Out of scope / not guaranteed
- Malware, keyloggers, screen capture or process injection while unlocked.
- A compromised OS, kernel, administrator or physical hardware.
- Secure erasure from SSD wear-leveling, snapshots, journals or backups.
- Protection from weak passwords or users who lose every recovery factor.
- Availability against disk failure, ransomware or deliberate deletion.
- Hiding all file sizes, timing, access patterns or filesystem metadata.
- Independent audit, FIPS validation or certification.

## Assumptions and mitigations
- The host OS and Rust toolchain are not malicious.
- nexusq-core and its cryptographic dependencies behave as documented.
- The OS random source is healthy.
- Users choose strong, unique vault and backup credentials.
- The app never silently downgrades cryptography or bypasses authentication.
- File writes use atomic create-only publication, restrictive permissions where supported, integrity verification and cleanup of partial output on reported errors.
- The app preserves source files by default; secure deletion is not promised.

## Important risks
- **Recovery compromise:** recovery materials can become a second master key. Separate backup credentials and document storage risks.
- **Plaintext residue:** applications, swap, crash dumps and filesystem snapshots may retain plaintext.
- **Metadata leakage:** folder names and relative paths are encrypted in the manifest, but object count, ciphertext sizes, timestamps and access patterns remain observable.
- **Supply chain:** a signed installer is only as trustworthy as its signing key, build pipeline and release process.
- **Quantum claims:** use only algorithms and constructions explicitly documented by the pinned nexusq-core release. Do not label the whole app quantum-proof.

## Security invariants
- No home-grown cryptographic primitives or key derivation.
- No unauthenticated plaintext output.
- No overwrite of an existing destination without explicit user consent.
- No destructive source deletion in the MVP.
- No secret material in logs, crash reports, telemetry or issue attachments.
- Format version and algorithm identifiers are explicit and validated.
- Backups are encrypted with credentials independent from the vault password.

## Review triggers
Any change to crypto APIs, file format, KDF parameters, key lifecycle, recovery, backup, temporary-file handling, permissions, update verification or dependency policy requires updated threat analysis and security approval.
