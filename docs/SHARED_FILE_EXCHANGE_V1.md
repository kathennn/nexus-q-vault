# Shared file exchange v1 — draft for security review

**Status:** proposed implementation draft; not independently security-reviewed or finalized.

## Protocol

A `.nqcontact` file is UTF-8 JSON with `format=nexusq-contact-card`, `version=1`, `identity_id`, display `label`, `key_id`, `algorithm=mlkem768-xwing-v1`, `public_key` (JSON byte array), and `fingerprint`.

The fingerprint is SHA-256 over the domain separator `NEXUS-Q CONTACT CARD V1\0` and length-prefixed identity ID, label, key ID and public key. It is not a signature. The UI requires the user to compare the full fingerprint with the recipient through a separate trusted channel before saving the contact as verified. The checkbox records the user's local attestation; it does not authenticate the card issuer.

The sender's verified-contact list is stored locally in the desktop webview's browser storage. It contains public contact-card data only, is not synchronized, and is not included in `.nqb` backups. Anyone with access to the same local application profile may be able to inspect it.

A shared `.nqx` package reuses the existing Core public-key envelope v1 (CBOR), hybrid ML-KEM-768 + X25519 encapsulation and AES-256-GCM payload protection. Envelope metadata is empty so filenames and paths are not disclosed. The format does not authenticate the sender.

## Key lifecycle and recovery

Each Vault receiving identity has a dedicated `MlKem768` / `KeyAgreement` key. Its private seed remains wrapped under the Vault KEK; the UI sees only the public half. `.nqb` backups include the identity-to-key reference and wrapped key material, re-wrapped under the new vault password during restore. The separate backup passphrase is required. The `.nqb` does not include `.nqx` packages or the local sender contact list.

A new receiving identity means a new contact card and fingerprint. Old active/rotating/retired keys remain usable for opening packages while retained; revoked/destroyed keys do not. Do not destroy old keys while packages addressed to them may still be needed.

## Limits and security properties

- One file and one recipient per package; plaintext limit 32 MiB, package overhead allowance 16 KiB.
- Empty files are supported. Inputs are bounded; symlink inputs are rejected.
- Output is atomically published without overwriting existing destinations. Originals are never modified.
- Malformed, truncated, unsupported-version/mode, wrong-recipient and tampered packages are rejected by Core.
- Successful decryption proves only that the selected receiving key opened the package and the AEAD tag was valid. It does not identify the sender.
- No server, sender signature, replay prevention, delivery receipt, multi-recipient support or forward secrecy against later compromise of the recipient's long-term key.
- If the receiving key and all usable `.nqb` backups are lost, packages addressed to it cannot be recovered.

## Review required before stabilization

Independently review the fingerprint construction and verification UX, Core envelope reuse, key lifecycle and backup behavior, metadata visibility, filesystem race/atomic-write behavior, key rotation/revocation and cross-platform support. This draft is not a security certification.
