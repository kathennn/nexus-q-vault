# Folder vault format — version 1

**Status:** prototype format. Not yet frozen for long-term compatibility or reviewed externally.

## Directory layout

A folder vault is a newly created directory:

```text
<folder-vault>/
  manifest.nqm
  objects/
    nqk_file_<128-bit-random-id>.nqx
    nqk_file_<128-bit-random-id>.nqx
```

The original directory is never modified or deleted. A restore writes to a new destination directory and refuses to overwrite existing files.

## Manifest envelope

`manifest.nqm` is a normal authenticated NEXUS-Q vault envelope produced by `Session::encrypt` in the pinned `nexusq-core` engine. The envelope's authenticated, public metadata is the fixed byte string:

`nexusq-vault-folder-manifest-v1`

It contains no original filename or path. The decrypted CBOR payload has this logical shape:

- `format_version`: unsigned integer, currently `1`.
- `entries`: array of file records:
  - `object_id`: opaque `nqk_file_...` identifier.
  - `relative_path`: portable UTF-8 relative path, with `/` separators.
  - `plaintext_size`: original file size in bytes.

The manifest is encrypted and authenticated using the vault's active AES-256-GCM key through `nexusq-core`. The application does not implement cryptographic primitives, key derivation, envelope encoding or a parallel encryption format.

## Object envelopes

Each file's ciphertext is stored as `objects/<object_id>.nqx`. Its object name contains a random identifier and does not reveal the original filename. The file contents are protected using the same active vault key through `Session::encrypt`. Its authenticated public metadata is the fixed prefix `nexusq-vault-folder-object-v1:` followed by the opaque object identifier; this binds the ciphertext to its storage name without exposing the original path. The object ID is referenced from the encrypted manifest.

The filesystem still reveals object count, approximate ciphertext sizes, timestamps and access patterns. This format does not claim to hide all metadata.

## Bounds and path policy in the prototype

- Maximum serialized manifest: 16 MiB.
- Maximum manifest entries: 100,000; maximum traversed filesystem entries: 200,000.
- Maximum directory nesting: 128; maximum component length: 255 bytes and 255 UTF-16 code units.
- Maximum plaintext size per file: 32 MiB, because the pinned engine reads complete files into memory.
- Paths must be relative UTF-8 and use the portable subset accepted by the validator; maximum path length is 4,096 bytes.
- Case-insensitive path collisions are rejected so common case-insensitive filesystems do not silently merge entries.
- Unicode normalization variants are not canonicalized yet; filesystems with normalization-insensitive naming require additional compatibility testing before beta.
- Absolute paths, parent traversal, drive/alternate-stream syntax, Windows-reserved names, invalid Windows characters, trailing dots/spaces and empty path components are rejected.
- Symbolic links and non-regular filesystem entries are rejected.
- Empty directories are not represented; directories are recreated as parents of files.
- The folder-vault root, object directory and restored subdirectories use mode `0700` on Unix; the manifest, object and restored files use mode `0600`. Windows ACL semantics still need review.

## Publication and restore behavior

Encryption requires a new output directory. Object envelopes are written using exclusive temporary files and create-only hard-link publication; the encrypted manifest is published last as the commit marker. On a reported error, the application attempts to remove the partial folder-vault directory. This is not a guarantee against power loss, disk failure, or a hostile process racing filesystem operations.

Restore authenticates and validates the manifest before creating a new destination. Each object is authenticated before plaintext publication. Outputs are create-only and temporary plaintext files use restrictive Unix permissions. If a reported error occurs, the application attempts to remove the partial destination; secure erasure is not guaranteed.

## Backup boundary

The existing `.nqb` backup contains vault/key state. It does **not** include individual `.nqx` sidecars, `manifest.nqm`, or folder-vault object directories. Users must preserve the encrypted file/folder payloads separately from the key backup. A future backup format may bundle these artifacts, but that requires its own versioned design and review.

## Version and migration policy

Version 1 readers reject unknown manifest versions. There is no automatic in-place migration yet. Before changing the schema, implement explicit migration tests and a recovery path that preserves the original folder vault. Until then, migration means decrypting with a compatible build and re-encrypting into a new folder vault. Never overwrite the only copy during migration.
