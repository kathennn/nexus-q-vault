# Streaming file envelope proposal — NQX2

**Status:** design proposal only. Not implemented. Requires review in the NEXUS-Q engine repository before any format or cryptographic code is merged.

## Why this is needed

The current NQX1 envelope stores one serialized ciphertext payload. The current file helpers read the complete source file and envelope into memory. NEXUS-Q Vault therefore caps individual files at 32 MiB and folder objects are also processed in memory.

Streaming must be implemented in `nexus-q`, not as a second encryption implementation in the desktop app. The app must continue to delegate cryptographic operations to `nexusq-core`.

## Compatibility and API direction

- Keep NQX1 decryption support indefinitely for existing vault files.
- Keep the existing in-memory `Session::encrypt` / `Session::decrypt` API for small structured payloads such as vault manifests.
- Add NQX2 streaming support to the core file API.
- New vault-mode file encryption writes NQX2; file decryption dispatches by magic and accepts both NQX1 and NQX2.
- Add explicit source-to-destination APIs so folder-vault objects can be streamed to opaque object paths without writing sidecars beside source files.
- Preserve the current key hierarchy: generate a fresh random DEK per file and wrap it using the existing KeyRecord/key-material wrapping path. Do not add another KDF, key store or crypto provider.
- Public-key/KEM streaming is out of scope for the first NQX2 implementation; it needs a separate design and test vectors.

## Proposed wire format

The following is a proposal; field widths, serialization and limits must be finalized in the engine ADR before implementation.

```
"NQX2"                 4 bytes, magic
header_length          u32 big-endian
header_bytes           bounded canonical CBOR
chunk_ciphertexts      exactly chunk_count records, no per-record length
footer_tag              one AEAD tag (16 bytes)
EOF                    no trailing bytes permitted
```

The header contains:
- format version and reserved flags;
- AEAD algorithm identifier and KeyId;
- the wrapped per-file DEK;
- random 4-byte nonce prefix;
- chunk size, declared plaintext size and derived chunk count;
- authenticated public metadata (for folder objects, the opaque object ID only).

Provisional parameters:
- 1 MiB plaintext chunk size;
- 64 GiB maximum plaintext size per file;
- chunk count derived as `ceil(plaintext_size / chunk_size)`;
- bounded header size (for example, 64 KiB).

The maximum file size and chunk size are provisional and require cryptographic review, particularly for AES-GCM per-key usage limits.

## Chunk authentication and nonce discipline

For chunk index `i`:
- nonce = random_nonce_prefix (4 bytes) || `i` (u64 big-endian);
- AAD = domain label `NQX2-CHUNK` || exact serialized header bytes || `i` || expected plaintext chunk length;
- encrypt/decrypt each chunk independently with the existing core AEAD primitive and the fresh per-file DEK.

The footer uses nonce index `chunk_count` (never used by a data chunk) and authenticates a distinct domain label, the exact header bytes, declared total size and chunk count by AEAD-encrypting an empty plaintext. This footer is mandatory even for an empty file.

The reader must:
1. validate magic, version, algorithm, header bounds, size bounds and the exact derived chunk count;
2. unwrap the DEK through the existing KeyRecord path;
3. read each expected ciphertext length exactly and authenticate chunks in order;
4. reject missing, reordered, duplicated or altered chunks;
5. authenticate the mandatory footer;
6. require immediate EOF and reject trailing bytes.

The exact serialized header bytes are AAD; the reader must not deserialize and reserialize a header to reconstruct AAD.

## Atomic file publication

- Encrypt into an exclusive same-directory temporary file; do not expose the final path while streaming.
- On encryption, verify the source is still the declared size and reject short reads or extra bytes.
- On decryption, write authenticated chunks only to a temporary plaintext file.
- Verify the footer and EOF before publishing any plaintext destination.
- Sync the temporary file, publish with the engine's create-only/no-clobber strategy, then sync the parent directory where supported.
- Remove temporary files on every error path. Never replace an existing destination.
- Keep plaintext chunks in zeroizing buffers; never accumulate the complete file in memory.

The engine implementation must define behavior for crashes between link/publication and temporary-file cleanup, disk-full errors, permission failures and filesystems that do not support the required create-only primitive.

## Folder-vault integration

After the engine API is reviewed and available:
- encrypt each source file directly to its opaque object path using NQX2;
- bind each object envelope's authenticated metadata to its opaque object ID, preserving the anti-swap check already required by the folder manifest;
- decrypt each object directly to a private temporary destination and publish only after its footer verifies;
- retain the encrypted manifest as a small authenticated in-memory envelope, subject to its existing manifest-size limit.

This removes the current 32 MiB per-file limitation from folder-vault objects without exposing names or paths in object metadata.

## Migration policy

- Never rewrite NQX1 files in place.
- Decrypt NQX1 to a new destination and encrypt NQX2 to a separate new destination.
- Keep the original until the user verifies the new file.
- The desktop app must clearly show the detected format and never claim that a file was upgraded unless both operations completed.
- NQX1 remains readable after NQX2 ships.

## Required tests and review gates

Before enabling NQX2 by default:
- independent review of nonce construction, key wrapping, AAD, footer and usage limits;
- published test vectors for empty, one-chunk, multi-chunk and boundary-size files;
- rejection tests for truncation at every frame boundary, reordered/duplicated chunks, modified header, metadata, ciphertext and footer, plus trailing bytes;
- source-file mutation tests during encryption;
- no-clobber, symlink, permissions, disk-full and crash/fault-injection tests;
- fuzzing of header parsing and chunk-count/length validation;
- cross-platform tests on Linux, Windows and macOS;
- backward-compatibility tests proving NQX1 remains readable;
- folder-vault tests proving object-ID binding survives streaming.

No NQX2 code should be merged into the engine until the format and its security properties have been reviewed. This proposal does not alter the v4.1.2 release or its artifacts.
