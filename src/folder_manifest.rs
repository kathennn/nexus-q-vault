//! Versioned encrypted folder manifest for NEXUS-Q Vault.
//!
//! Paths and file sizes live only inside an authenticated envelope created by
//! the pinned nexusq-core session. The on-disk object name is an opaque KeyId;
//! this module does not implement cryptography or filesystem publication.
use nexusq_core::crypto::random::OsRandomSource;
use nexusq_core::vault::{Envelope, KeyId, Session, serde_helpers};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io;
use zeroize::{Zeroize, Zeroizing};

pub const MANIFEST_VERSION: u16 = 1;
pub const MAX_MANIFEST_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_MANIFEST_ENTRIES: usize = 100_000;
pub const MAX_FILESYSTEM_ENTRIES: usize = 200_000;
const ENVELOPE_METADATA: &[u8] = b"nexusq-vault-folder-manifest-v1";
const OBJECT_METADATA_PREFIX: &str = "nexusq-vault-folder-object-v1:";

type ManifestResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// A single protected file. Its storage object is named by `object_id`, not
/// by the original filename. The original relative path is encrypted.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub object_id: String,
    pub relative_path: String,
    pub plaintext_size: u64,
}

impl std::fmt::Debug for ManifestEntry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ManifestEntry")
            .field("object_id", &self.object_id)
            .field("relative_path", &"[REDACTED]")
            .field("plaintext_size", &self.plaintext_size)
            .finish()
    }
}

impl Drop for ManifestEntry {
    fn drop(&mut self) {
        self.object_id.zeroize();
        self.relative_path.zeroize();
    }
}

/// Encrypted logical view of one folder. Serialize only inside an envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderManifest {
    pub format_version: u16,
    pub entries: Vec<ManifestEntry>,
}

impl Default for FolderManifest {
    fn default() -> Self {
        Self::new()
    }
}

impl FolderManifest {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            format_version: MANIFEST_VERSION,
            entries: Vec::new(),
        }
    }

    /// Add a file using a portable relative path and return its opaque object id.
    pub fn add_file(&mut self, relative_path: &str, plaintext_size: u64) -> ManifestResult<String> {
        validate_relative_path(relative_path)?;
        if self.entries.len() >= MAX_MANIFEST_ENTRIES {
            return Err(invalid("manifest entry limit exceeded"));
        }
        if plaintext_size > MAX_FILE_BYTES {
            return Err(invalid("file exceeds the current per-file size limit"));
        }
        let folded_path = relative_path.to_lowercase();
        if self
            .entries
            .iter()
            .any(|entry| entry.relative_path.to_lowercase() == folded_path)
        {
            return Err(invalid(
                "duplicate relative path or case-insensitive path collision",
            ));
        }

        let id = KeyId::generate(&OsRandomSource::new(), "file")?.to_string();
        self.entries.push(ManifestEntry {
            object_id: id.clone(),
            relative_path: relative_path.to_owned(),
            plaintext_size,
        });
        if let Err(error) = self.validate() {
            self.entries.pop();
            return Err(error);
        }
        Ok(id)
    }

    /// Validate version, resource bounds, portable paths and unique identifiers.
    pub fn validate(&self) -> ManifestResult<()> {
        if self.format_version != MANIFEST_VERSION {
            return Err(invalid("unsupported folder manifest version"));
        }
        if self.entries.len() > MAX_MANIFEST_ENTRIES {
            return Err(invalid("manifest entry limit exceeded"));
        }

        let mut ids = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for entry in &self.entries {
            validate_relative_path(&entry.relative_path)?;
            let parsed: KeyId = entry.object_id.parse()?;
            if parsed.algorithm() != "file" {
                return Err(invalid("invalid opaque object id type"));
            }
            if entry.plaintext_size > MAX_FILE_BYTES {
                return Err(invalid(
                    "manifest entry exceeds the current per-file size limit",
                ));
            }
            if !ids.insert(entry.object_id.as_str()) {
                return Err(invalid("duplicate object id"));
            }
            if !paths.insert(entry.relative_path.to_lowercase()) {
                return Err(invalid(
                    "duplicate relative path or case-insensitive path collision",
                ));
            }
        }
        Ok(())
    }

    /// Encrypt this manifest as a nexusq-core envelope with no filename metadata.
    pub fn seal(&self, session: &Session, key_id: &KeyId) -> ManifestResult<Vec<u8>> {
        self.validate()?;
        let plaintext = Zeroizing::new(serde_helpers::to_vec(self)?);
        if plaintext.len() > MAX_MANIFEST_BYTES {
            return Err(invalid("manifest exceeds maximum serialized size"));
        }
        Ok(session.encrypt(key_id, &plaintext, ENVELOPE_METADATA.to_vec())?)
    }

    /// Authenticate and decrypt a manifest envelope, then validate all entries.
    pub fn open(session: &Session, envelope: &[u8]) -> ManifestResult<Self> {
        if envelope.is_empty() || envelope.len() > MAX_MANIFEST_BYTES + 64 * 1024 {
            return Err(invalid("manifest envelope size is invalid"));
        }
        let parsed: Envelope = serde_helpers::from_slice(envelope)?;
        if parsed.header.metadata != ENVELOPE_METADATA {
            return Err(invalid("envelope is not a NEXUS-Q folder manifest"));
        }
        drop(parsed);
        let plaintext = session.decrypt(envelope)?;
        if plaintext.len() > MAX_MANIFEST_BYTES {
            return Err(invalid("decrypted manifest exceeds maximum size"));
        }
        let manifest: Self = serde_helpers::from_slice(plaintext.as_slice())?;
        manifest.validate()?;
        Ok(manifest)
    }
}

fn validate_relative_path(path: &str) -> ManifestResult<()> {
    if path.is_empty()
        || path.len() > 4096
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains(':')
        || path.contains('\0')
    {
        return Err(invalid("manifest path must be a portable relative path"));
    }
    let components: Vec<&str> = path.split('/').collect();
    const WINDOWS_RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    for part in components {
        if part.is_empty() || part == "." || part == ".." {
            return Err(invalid("manifest path contains an unsafe component"));
        }
        if part.len() > 255
            || part.encode_utf16().count() > 255
            || part
                .chars()
                .any(|ch| ch.is_control() || matches!(ch, '<' | '>' | '"' | '|' | '?' | '*'))
            || part.ends_with(['.', ' '])
            || WINDOWS_RESERVED.contains(
                &part
                    .split('.')
                    .next()
                    .unwrap_or("")
                    .to_ascii_uppercase()
                    .as_str(),
            )
        {
            return Err(invalid(
                "manifest path is not portable across desktop platforms",
            ));
        }
    }
    Ok(())
}

fn invalid(message: &'static str) -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(io::Error::new(io::ErrorKind::InvalidData, message))
}

fn object_metadata(object_id: &str) -> Vec<u8> {
    format!("{OBJECT_METADATA_PREFIX}{object_id}").into_bytes()
}

fn decrypt_object(
    session: &Session,
    object_id: &str,
    envelope: &[u8],
) -> ManifestResult<Zeroizing<Vec<u8>>> {
    let parsed: Envelope = serde_helpers::from_slice(envelope)?;
    if parsed.header.metadata != object_metadata(object_id) {
        return Err(invalid(
            "encrypted object does not match its manifest identifier",
        ));
    }
    drop(parsed);
    Ok(session.decrypt(envelope)?)
}

/// Current in-memory engine limit for one file operation. The pinned core reads
/// complete files into memory; larger files are refused rather than risking
/// unbounded memory use. Streaming support belongs in nexusq-core.
pub const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;

/// Encrypt a directory into a new folder-vault directory.
///
/// The output contains `manifest.nqm` and `objects/<opaque-id>.nqx`. Source
/// files are never modified. Symlinks and non-regular filesystem entries are
/// rejected. The manifest is published last, so an incomplete output is not
/// mistaken for a complete folder vault.
pub fn encrypt_folder(
    session: &Session,
    key_id: &KeyId,
    source_dir: &std::path::Path,
    vault_dir: &std::path::Path,
) -> ManifestResult<usize> {
    let source = source_dir.canonicalize()?;
    if !source.is_dir() {
        return Err(invalid("source path is not a directory"));
    }
    create_private_dir(vault_dir)?;
    let result = (|| {
        let vault_root = vault_dir.canonicalize()?;
        if vault_root.starts_with(&source) {
            return Err(invalid("folder vault cannot be created inside its source"));
        }
        let objects_dir = vault_root.join("objects");
        create_private_dir(&objects_dir)?;
        let mut files = Vec::new();
        let mut visited_entries = 0;
        collect_regular_files(&source, &mut files, 0, &mut visited_entries)?;
        if files.len() > MAX_MANIFEST_ENTRIES {
            return Err(invalid("folder contains too many files"));
        }

        let mut manifest = FolderManifest::new();
        for file in files {
            let relative = file.strip_prefix(&source)?;
            let relative_path = Zeroizing::new(
                relative
                    .to_str()
                    .ok_or_else(|| invalid("non-UTF-8 file paths are not supported"))?
                    .replace(std::path::MAIN_SEPARATOR, "/"),
            );
            let metadata = std::fs::metadata(&file)?;
            if metadata.len() > MAX_FILE_BYTES {
                return Err(invalid("file exceeds the current 32 MiB in-memory limit"));
            }
            let plaintext = Zeroizing::new(std::fs::read(&file)?);
            if plaintext.len() as u64 > MAX_FILE_BYTES || plaintext.len() as u64 != metadata.len() {
                return Err(invalid(
                    "file changed while it was being encrypted or exceeds the size limit",
                ));
            }
            let object_id = manifest.add_file(&relative_path, plaintext.len() as u64)?;
            let envelope = session.encrypt(key_id, &plaintext, object_metadata(&object_id))?;
            let object_path = objects_dir.join(format!("{object_id}.nqx"));
            write_new_atomic(&object_path, &envelope)?;
        }

        let sealed_manifest = manifest.seal(session, key_id)?;
        write_new_atomic(&vault_root.join("manifest.nqm"), &sealed_manifest)?;
        Ok(manifest.entries.len())
    })();

    if result.is_err() {
        let _ = std::fs::remove_dir_all(vault_dir);
    }
    result
}

/// Restore a folder vault into a new destination directory.
///
/// Every path is revalidated after decrypting the manifest, every envelope is
/// authenticated by nexusq-core, and each plaintext output is create-only.
pub fn decrypt_folder(
    session: &Session,
    vault_dir: &std::path::Path,
    destination_dir: &std::path::Path,
) -> ManifestResult<usize> {
    let vault_root = vault_dir.canonicalize()?;
    let manifest_path = vault_root.join("manifest.nqm");
    let manifest_metadata = std::fs::symlink_metadata(&manifest_path)?;
    if manifest_metadata.file_type().is_symlink() || !manifest_metadata.is_file() {
        return Err(invalid("folder manifest must be a regular file"));
    }
    let manifest_size = manifest_metadata.len();
    if manifest_size > (MAX_MANIFEST_BYTES + 64 * 1024) as u64 {
        return Err(invalid("manifest envelope exceeds maximum size"));
    }
    let envelope = std::fs::read(&manifest_path)?;
    let manifest = FolderManifest::open(session, &envelope)?;

    create_private_dir(destination_dir)?;
    let result = (|| {
        let destination = destination_dir.canonicalize()?;
        let objects_dir = vault_root.join("objects");
        let objects_metadata = std::fs::symlink_metadata(&objects_dir)?;
        if objects_metadata.file_type().is_symlink() || !objects_metadata.is_dir() {
            return Err(invalid("folder objects path must be a real directory"));
        }
        for entry in &manifest.entries {
            validate_relative_path(&entry.relative_path)?;
            let object_path = objects_dir.join(format!("{}.nqx", entry.object_id));
            let object_metadata = std::fs::symlink_metadata(&object_path)?;
            if object_metadata.file_type().is_symlink() || !object_metadata.is_file() {
                return Err(invalid("encrypted object must be a regular file"));
            }
            let object_size = object_metadata.len();
            if object_size > MAX_FILE_BYTES + 1024 * 1024 {
                return Err(invalid("encrypted file exceeds maximum size"));
            }
            let encrypted = std::fs::read(&object_path)?;
            let plaintext = decrypt_object(session, &entry.object_id, &encrypted)?;
            if plaintext.len() as u64 != entry.plaintext_size {
                return Err(invalid("decrypted file size does not match the manifest"));
            }
            let relative = std::path::Path::new(&entry.relative_path);
            let output = destination.join(relative);
            let parent = output
                .parent()
                .ok_or_else(|| invalid("invalid output path"))?;
            create_private_dir_all(&destination, parent)?;
            write_new_atomic(&output, plaintext.as_slice())?;
        }
        Ok(manifest.entries.len())
    })();

    if result.is_err() {
        let _ = std::fs::remove_dir_all(destination_dir);
    }
    result
}

fn collect_regular_files(
    current: &std::path::Path,
    output: &mut Vec<std::path::PathBuf>,
    depth: usize,
    visited_entries: &mut usize,
) -> ManifestResult<()> {
    if depth > 128 {
        return Err(invalid("folder nesting exceeds the 128-directory limit"));
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(current)? {
        *visited_entries += 1;
        if *visited_entries > MAX_FILESYSTEM_ENTRIES {
            return Err(invalid("folder contains too many filesystem entries"));
        }
        entries.push(entry?);
    }
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(invalid("symbolic links are not supported in folder vaults"));
        }
        if kind.is_dir() {
            collect_regular_files(&path, output, depth + 1, visited_entries)?;
        } else if kind.is_file() {
            output.push(path);
        } else {
            return Err(invalid("unsupported filesystem entry in source folder"));
        }
        if output.len() > MAX_MANIFEST_ENTRIES {
            return Err(invalid("folder contains too many files"));
        }
    }
    Ok(())
}

fn create_private_dir(path: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn create_private_dir_all(root: &std::path::Path, path: &std::path::Path) -> std::io::Result<()> {
    let relative = path.strip_prefix(root).map_err(std::io::Error::other)?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        match std::fs::create_dir(&current) {
            Ok(()) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&current, std::fs::Permissions::from_mode(0o700))?;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let metadata = std::fs::symlink_metadata(&current)?;
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "restore path contains a non-directory or symbolic link",
                    ));
                }
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn write_new_atomic(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    let file_name = path.file_name().unwrap_or_default();
    let (temp_path, mut file) = loop {
        let suffix = KeyId::generate(&OsRandomSource::new(), "tmp")
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        let mut name = file_name.to_os_string();
        name.push(format!(".{suffix}.tmp"));
        let candidate = parent.join(name);
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&candidate) {
            Ok(file) => break (candidate, file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };

    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::hard_link(&temp_path, path)?;
        std::fs::remove_file(&temp_path)?;
        #[cfg(unix)]
        {
            let directory = std::fs::File::open(parent)?;
            directory.sync_all()?;
        }
        Ok(())
    })();
    let _ = std::fs::remove_file(&temp_path);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexusq_core::vault::{Algorithm, Purpose, Vault};
    use tempfile::tempdir;

    fn unlocked_session(dir: &std::path::Path) -> (Session, KeyId) {
        let path = dir.join("manifest-test.nqv");
        let password = b"manifest-test-password";
        let vault = Vault::create(&path, password, Some("manifest test".into())).unwrap();
        let mut session = vault.unlock(password).unwrap();
        let key_id = session
            .generate_key(Algorithm::Aes256Gcm, Purpose::Encrypt)
            .unwrap();
        session.activate_key(&key_id).unwrap();
        (session, key_id)
    }

    #[test]
    fn manifest_roundtrip_encrypts_names_and_structure() {
        let dir = tempdir().unwrap();
        let (session, key_id) = unlocked_session(dir.path());
        let mut manifest = FolderManifest::new();
        manifest
            .add_file("Personal/Tax return 2026.pdf", 1234)
            .unwrap();
        manifest
            .add_file("Photos/holiday/image 01.jpg", 9876)
            .unwrap();

        let sealed = manifest.seal(&session, &key_id).unwrap();
        assert!(
            !sealed
                .windows(b"Tax return 2026.pdf".len())
                .any(|w| w == b"Tax return 2026.pdf")
        );
        assert!(!sealed.windows(b"Photos".len()).any(|w| w == b"Photos"));

        let opened = FolderManifest::open(&session, &sealed).unwrap();
        assert_eq!(opened, manifest);
    }

    #[test]
    fn manifest_rejects_path_traversal_and_nonportable_paths() {
        let dir = tempdir().unwrap();
        let mut manifest = FolderManifest::new();
        assert!(manifest.add_file("../outside.txt", 1).is_err());
        assert!(manifest.add_file("/absolute/path.txt", 1).is_err());
        assert!(manifest.add_file("folder\\..\\outside.txt", 1).is_err());
        assert!(manifest.add_file("C:/outside.txt", 1).is_err());
        assert!(manifest.add_file("CON.txt", 1).is_err());
        assert!(manifest.add_file("trailing-dot.", 1).is_err());
        assert!(manifest.add_file("bad?.txt", 1).is_err());
        assert!(manifest.entries.is_empty());
        let _ = dir;
    }

    #[test]
    fn manifest_rejects_duplicate_paths() {
        let mut manifest = FolderManifest::new();
        manifest.add_file("same/name.txt", 1).unwrap();
        assert!(manifest.add_file("same/name.txt", 2).is_err());
        assert!(manifest.add_file("SAME/NAME.TXT", 2).is_err());
    }

    #[test]
    fn tampered_manifest_envelope_is_rejected() {
        let dir = tempdir().unwrap();
        let (session, key_id) = unlocked_session(dir.path());
        let mut manifest = FolderManifest::new();
        manifest.add_file("secret.txt", 10).unwrap();
        let mut sealed = manifest.seal(&session, &key_id).unwrap();
        let last = sealed.len() - 1;
        sealed[last] ^= 0x40;
        assert!(FolderManifest::open(&session, &sealed).is_err());
    }

    #[test]
    fn manifest_rejects_unknown_format_version() {
        let mut manifest = FolderManifest::new();
        manifest.format_version = MANIFEST_VERSION + 1;
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn folder_encryption_hides_names_and_restores_tree() {
        let dir = tempdir().unwrap();
        let (session, key_id) = unlocked_session(dir.path());
        let source = dir.path().join("source");
        let vault_dir = dir.path().join("protected-folder");
        let restored = dir.path().join("restored");
        std::fs::create_dir_all(source.join("Personal/Taxes")).unwrap();
        std::fs::write(
            source.join("Personal/Taxes/tax-return-2026.txt"),
            b"tax payload",
        )
        .unwrap();
        std::fs::write(source.join("readme.txt"), b"hello vault").unwrap();

        let count = encrypt_folder(&session, &key_id, &source, &vault_dir).unwrap();
        assert_eq!(count, 2);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&vault_dir).unwrap().permissions().mode() & 0o777,
                0o700
            );
            assert_eq!(
                std::fs::metadata(vault_dir.join("objects"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
            let object = std::fs::read_dir(vault_dir.join("objects"))
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            assert_eq!(
                std::fs::metadata(object).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let manifest_bytes = std::fs::read(vault_dir.join("manifest.nqm")).unwrap();
        assert!(
            !manifest_bytes
                .windows(b"tax-return-2026.txt".len())
                .any(|w| w == b"tax-return-2026.txt")
        );
        let object_names = std::fs::read_dir(vault_dir.join("objects"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(
            object_names
                .iter()
                .all(|name| !name.contains("readme") && !name.contains("tax-return"))
        );

        let restored_count = decrypt_folder(&session, &vault_dir, &restored).unwrap();
        assert_eq!(restored_count, 2);
        assert_eq!(
            std::fs::read(restored.join("Personal/Taxes/tax-return-2026.txt")).unwrap(),
            b"tax payload"
        );
        assert_eq!(
            std::fs::read(restored.join("readme.txt")).unwrap(),
            b"hello vault"
        );
        assert_eq!(
            std::fs::read(source.join("readme.txt")).unwrap(),
            b"hello vault"
        );
    }

    #[cfg(unix)]
    #[test]
    fn folder_encryption_rejects_symlinks_and_cleans_partial_output() {
        use std::os::unix::fs::symlink;
        let dir = tempdir().unwrap();
        let (session, key_id) = unlocked_session(dir.path());
        let source = dir.path().join("source-with-link");
        let vault_dir = dir.path().join("should-be-removed");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("real.txt"), b"safe").unwrap();
        symlink(source.join("real.txt"), source.join("link.txt")).unwrap();

        assert!(encrypt_folder(&session, &key_id, &source, &vault_dir).is_err());
        assert!(!vault_dir.exists());
    }

    #[test]
    fn manifest_rejects_an_authenticated_non_manifest_envelope() {
        let dir = tempdir().unwrap();
        let (session, key_id) = unlocked_session(dir.path());
        let manifest = FolderManifest::new();
        let plaintext = serde_helpers::to_vec(&manifest).unwrap();
        let ordinary_file_envelope = session.encrypt(&key_id, &plaintext, Vec::new()).unwrap();
        assert!(FolderManifest::open(&session, &ordinary_file_envelope).is_err());
    }

    #[test]
    fn folder_restore_rejects_swapped_objects() {
        let dir = tempdir().unwrap();
        let (session, key_id) = unlocked_session(dir.path());
        let source = dir.path().join("source");
        let vault_dir = dir.path().join("vault-folder");
        let restored = dir.path().join("swapped-restore");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("a.txt"), b"alpha").unwrap();
        std::fs::write(source.join("b.txt"), b"bravo").unwrap();
        encrypt_folder(&session, &key_id, &source, &vault_dir).unwrap();

        let manifest = FolderManifest::open(
            &session,
            &std::fs::read(vault_dir.join("manifest.nqm")).unwrap(),
        )
        .unwrap();
        let first = vault_dir
            .join("objects")
            .join(format!("{}.nqx", manifest.entries[0].object_id));
        let second = vault_dir
            .join("objects")
            .join(format!("{}.nqx", manifest.entries[1].object_id));
        let first_bytes = std::fs::read(&first).unwrap();
        let second_bytes = std::fs::read(&second).unwrap();
        std::fs::write(&first, second_bytes).unwrap();
        std::fs::write(&second, first_bytes).unwrap();

        assert!(decrypt_folder(&session, &vault_dir, &restored).is_err());
        assert!(!restored.exists());
    }

    #[cfg(unix)]
    #[test]
    fn folder_restore_rejects_object_symlinks() {
        use std::os::unix::fs::symlink;
        let dir = tempdir().unwrap();
        let (session, key_id) = unlocked_session(dir.path());
        let source = dir.path().join("source");
        let vault_dir = dir.path().join("vault-folder");
        let restored = dir.path().join("symlink-restore");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("private.txt"), b"private").unwrap();
        encrypt_folder(&session, &key_id, &source, &vault_dir).unwrap();

        let manifest = FolderManifest::open(
            &session,
            &std::fs::read(vault_dir.join("manifest.nqm")).unwrap(),
        )
        .unwrap();
        let object = vault_dir
            .join("objects")
            .join(format!("{}.nqx", manifest.entries[0].object_id));
        let external = dir.path().join("external-object.nqx");
        std::fs::rename(&object, &external).unwrap();
        symlink(&external, &object).unwrap();

        assert!(decrypt_folder(&session, &vault_dir, &restored).is_err());
        assert!(!restored.exists());
    }

    #[test]
    fn tampered_folder_object_fails_closed_and_removes_partial_restore() {
        let dir = tempdir().unwrap();
        let (session, key_id) = unlocked_session(dir.path());
        let source = dir.path().join("source");
        let vault_dir = dir.path().join("vault-folder");
        let restored = dir.path().join("partial-restore");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("one.txt"), b"one").unwrap();
        std::fs::write(source.join("two.txt"), b"two").unwrap();
        encrypt_folder(&session, &key_id, &source, &vault_dir).unwrap();

        let manifest = FolderManifest::open(
            &session,
            &std::fs::read(vault_dir.join("manifest.nqm")).unwrap(),
        )
        .unwrap();
        let object = vault_dir
            .join("objects")
            .join(format!("{}.nqx", manifest.entries[0].object_id));
        let mut bytes = std::fs::read(&object).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        std::fs::write(object, bytes).unwrap();

        assert!(decrypt_folder(&session, &vault_dir, &restored).is_err());
        assert!(!restored.exists());
    }
}
