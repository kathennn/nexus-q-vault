use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use nexusq_core::{crypto::hash::sha256, identity::IdentityId};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

pub const MAX_SHARED_FILE_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_SHARED_PACKAGE_BYTES: u64 = MAX_SHARED_FILE_BYTES + 16 * 1024;
pub const MAX_CONTACT_CARD_BYTES: u64 = 64 * 1024;
const CONTACT_CARD_FORMAT: &str = "nexusq-contact-card";
const CONTACT_CARD_VERSION: u8 = 1;
const CONTACT_CARD_ALGORITHM: &str = "mlkem768-xwing-v1";
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactCard {
    pub format: String,
    pub version: u8,
    pub identity_id: String,
    pub label: String,
    pub key_id: String,
    pub algorithm: String,
    pub public_key: Vec<u8>,
    pub fingerprint: String,
}

impl ContactCard {
    pub fn new(
        identity_id: String,
        label: String,
        key_id: String,
        public_key: Vec<u8>,
    ) -> Result<Self, String> {
        let mut card = Self {
            format: CONTACT_CARD_FORMAT.to_string(),
            version: CONTACT_CARD_VERSION,
            identity_id,
            label,
            key_id,
            algorithm: CONTACT_CARD_ALGORITHM.to_string(),
            public_key,
            fingerprint: String::new(),
        };
        card.fingerprint = card.computed_fingerprint();
        card.validate()?;
        Ok(card)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.format != CONTACT_CARD_FORMAT || self.version != CONTACT_CARD_VERSION {
            return Err("Unsupported contact card format or version".into());
        }
        if self.algorithm != CONTACT_CARD_ALGORITHM {
            return Err("The contact card uses an unsupported key algorithm".into());
        }
        self.identity_id
            .parse::<IdentityId>()
            .map_err(|_| "The contact card has an invalid identity identifier".to_string())?;
        if self.label.trim().is_empty()
            || self.label.len() > 80
            || self.label.chars().any(char::is_control)
        {
            return Err("The contact card has an invalid display name".into());
        }
        let parsed_key_id = self
            .key_id
            .parse::<nexusq_core::vault::KeyId>()
            .map_err(|_| "The contact card has an invalid key identifier".to_string())?;
        if parsed_key_id.algorithm() != "mlkem768" {
            return Err("The contact card key is not ML-KEM-768".into());
        }
        if self.public_key.len() != nexusq_core::crypto::kem::hybrid::PUBLIC_KEY_LEN {
            return Err("The contact card public key has an invalid length".into());
        }
        if self.fingerprint != self.computed_fingerprint() {
            return Err("The contact card fingerprint does not match its contents".into());
        }
        Ok(())
    }

    fn computed_fingerprint(&self) -> String {
        let mut input = Vec::with_capacity(
            32 + self.identity_id.len()
                + self.label.len()
                + self.key_id.len()
                + self.public_key.len(),
        );
        input.extend_from_slice(b"NEXUS-Q CONTACT CARD V1\0");
        input.extend_from_slice(&(self.identity_id.len() as u32).to_be_bytes());
        input.extend_from_slice(self.identity_id.as_bytes());
        input.extend_from_slice(&(self.label.len() as u32).to_be_bytes());
        input.extend_from_slice(self.label.as_bytes());
        input.extend_from_slice(&(self.key_id.len() as u32).to_be_bytes());
        input.extend_from_slice(self.key_id.as_bytes());
        input.extend_from_slice(&(self.public_key.len() as u32).to_be_bytes());
        input.extend_from_slice(&self.public_key);
        format!("sha256:{}", encode_hex(&sha256(&input)))
    }
}

pub fn read_contact_card(path: impl AsRef<Path>) -> Result<ContactCard, String> {
    let bytes = read_bounded_file(path, MAX_CONTACT_CARD_BYTES, "contact card")?;
    let card: ContactCard = serde_json::from_slice(&bytes)
        .map_err(|_| "The selected file is not a valid NEXUS-Q contact card".to_string())?;
    card.validate()?;
    Ok(card)
}

pub fn read_bounded_file(
    path: impl AsRef<Path>,
    max_bytes: u64,
    description: &str,
) -> Result<Zeroizing<Vec<u8>>, String> {
    let path = path.as_ref();
    let path_metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("Could not inspect the {description}: {error}"))?;
    if path_metadata.file_type().is_symlink() || !path_metadata.is_file() {
        return Err(format!(
            "The {description} must be a regular file, not a symbolic link"
        ));
    }
    if path_metadata.len() > max_bytes {
        return Err(format!(
            "The {description} exceeds the supported size limit"
        ));
    }
    let mut file =
        File::open(path).map_err(|error| format!("Could not open the {description}: {error}"))?;
    let mut bytes = Zeroizing::new(Vec::with_capacity(path_metadata.len() as usize));
    Read::by_ref(&mut file)
        .take(max_bytes.saturating_add(1))
        .read_to_end(bytes.as_mut())
        .map_err(|error| format!("Could not read the {description}: {error}"))?;
    if bytes.len() as u64 > max_bytes {
        return Err(format!(
            "The {description} exceeds the supported size limit"
        ));
    }
    Ok(bytes)
}

pub fn create_shared_package_file(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    card: &ContactCard,
    mut cancelled: impl FnMut() -> bool,
) -> Result<(), String> {
    card.validate()?;
    if cancelled() {
        return Err("operation cancelled".into());
    }
    let plaintext = read_bounded_file(input_path, MAX_SHARED_FILE_BYTES, "source file")?;
    if cancelled() {
        return Err("operation cancelled".into());
    }
    let package = nexusq_core::vault::envelope::build_envelope_to_public_key(
        &card.public_key,
        plaintext.as_slice(),
        Vec::new(),
    )
    .map_err(|error| format!("Could not encrypt the shared package: {error}"))?;
    if package.len() as u64 > MAX_SHARED_PACKAGE_BYTES {
        return Err("The encrypted package exceeds the supported size limit".into());
    }
    if cancelled() {
        return Err("operation cancelled".into());
    }
    write_new_atomic(output_path, &package)
}

pub fn write_new_atomic(path: impl AsRef<Path>, bytes: &[u8]) -> Result<(), String> {
    let path = path.as_ref();
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let file_name = path.file_name().unwrap_or_default();

    for _ in 0..16 {
        let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut temp_name = file_name.to_os_string();
        temp_name.push(format!(".tmp-{}-{timestamp}-{counter}", std::process::id()));
        let temp_path = parent.join(temp_name);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = match options.open(&temp_path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("Could not create temporary output: {error}")),
        };

        let mut published = false;
        let result = (|| -> std::io::Result<()> {
            file.write_all(bytes)?;
            file.sync_all()?;
            drop(file);
            fs::hard_link(&temp_path, path)?;
            published = true;
            fs::remove_file(&temp_path)?;
            #[cfg(unix)]
            {
                let directory = File::open(parent)?;
                match directory.sync_all() {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::Unsupported => {}
                    Err(error) => return Err(error),
                }
            }
            Ok(())
        })();
        if result.is_err() {
            let mut cleanup_errors = Vec::new();
            if published
                && let Err(error) = fs::remove_file(path)
                && error.kind() != std::io::ErrorKind::NotFound
            {
                cleanup_errors.push(error.to_string());
            }
            if let Err(error) = fs::remove_file(&temp_path)
                && error.kind() != std::io::ErrorKind::NotFound
            {
                cleanup_errors.push(error.to_string());
            }
            if !cleanup_errors.is_empty() {
                return Err(format!(
                    "Output publication failed and cleanup also failed: {}",
                    cleanup_errors.join("; ")
                ));
            }
        } else {
            let _ = fs::remove_file(&temp_path);
        }
        return result.map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                "The selected destination already exists; no file was overwritten".to_string()
            } else {
                format!("Could not publish the output file: {error}")
            }
        });
    }
    Err("Could not allocate a unique temporary output file".into())
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexusq_core::crypto::kem::hybrid;
    use tempfile::TempDir;

    fn sample_card() -> ContactCard {
        let key_pair = hybrid::generate();
        ContactCard::new(
            "nqi_0123456789abcdef0123456789abcdef".into(),
            "Alice".into(),
            "nqk_mlkem768_0123456789abcdef0123456789abcdef".into(),
            key_pair.public_key_bytes(),
        )
        .unwrap()
    }

    #[test]
    fn contact_card_roundtrips_and_detects_tampering() {
        let card = sample_card();
        let encoded = serde_json::to_vec(&card).unwrap();
        let decoded: ContactCard = serde_json::from_slice(&encoded).unwrap();
        decoded.validate().unwrap();

        let mut tampered = decoded;
        tampered.public_key[0] ^= 1;
        assert!(tampered.validate().is_err());
    }

    #[test]
    fn contact_card_file_is_bounded_and_roundtrips() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("recipient.nqcontact");
        let card = sample_card();
        let encoded = serde_json::to_vec_pretty(&card).unwrap();
        write_new_atomic(&path, &encoded).unwrap();
        let decoded = read_contact_card(&path).unwrap();
        assert_eq!(decoded, card);
    }

    #[test]
    fn contact_card_rejects_unknown_versions_and_wrong_key_ids() {
        let mut card = sample_card();
        card.version = 2;
        assert!(card.validate().is_err());

        let mut card = sample_card();
        card.key_id = "nqk_ed25519_0123456789abcdef0123456789abcdef".into();
        card.fingerprint = card.computed_fingerprint();
        assert!(card.validate().is_err());
    }

    #[test]
    fn bounded_reader_rejects_oversized_files_and_symlinks() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("card.json");
        fs::write(&file, b"12345").unwrap();
        assert!(read_bounded_file(&file, 4, "test file").is_err());
        #[cfg(unix)]
        {
            let link = dir.path().join("link.json");
            std::os::unix::fs::symlink(&file, &link).unwrap();
            assert!(read_bounded_file(&link, 10, "test file").is_err());
        }
    }

    #[test]
    fn independent_vaults_exchange_a_file_and_backup_restores_receiving_keys() {
        use nexusq_core::{
            storage::backup,
            vault::{Algorithm, Purpose, Vault},
        };

        let dir = TempDir::new().unwrap();
        let recipient_path = dir.path().join("recipient.nqv");
        let recipient_vault = Vault::create(&recipient_path, b"recipient-password", None).unwrap();
        let mut recipient = recipient_vault.unlock(b"recipient-password").unwrap();
        let identity_id = recipient.create_identity(Some("Alice".into())).unwrap();
        let key_id = recipient
            .generate_key(Algorithm::MlKem768, Purpose::KeyAgreement)
            .unwrap();
        recipient.activate_key(&key_id).unwrap();
        recipient
            .find_identity_mut(&identity_id)
            .unwrap()
            .unwrap()
            .key_agreement_key = Some(key_id.clone());
        let public_key = recipient
            .find_key(&key_id)
            .unwrap()
            .public_key_bytes()
            .unwrap()
            .to_vec();
        let card = ContactCard::new(
            identity_id.to_string(),
            "Alice".into(),
            key_id.to_string(),
            public_key,
        )
        .unwrap();

        let sender_path = dir.path().join("sender.nqv");
        let sender_vault = Vault::create(&sender_path, b"sender-password", None).unwrap();
        let sender = sender_vault.unlock(b"sender-password").unwrap();
        assert_eq!(sender.key_count(), 0);
        let original = b"byte-for-byte shared payload\0\xff";
        let source_path = dir.path().join("source-ñ.txt");
        let package_path = dir.path().join("shared-file.nqx");
        fs::write(&source_path, original).unwrap();
        create_shared_package_file(&source_path, &package_path, &card, || false).unwrap();
        assert_eq!(fs::read(&source_path).unwrap(), original);
        let package =
            read_bounded_file(&package_path, MAX_SHARED_PACKAGE_BYTES, "shared package").unwrap();
        let recovered = recipient
            .open_envelope_with_key_agreement(&key_id, package.as_slice())
            .unwrap();
        assert_eq!(recovered.as_slice(), original);

        let wrong_key_id = recipient
            .generate_key(Algorithm::MlKem768, Purpose::KeyAgreement)
            .unwrap();
        recipient.activate_key(&wrong_key_id).unwrap();
        assert!(
            recipient
                .open_envelope_with_key_agreement(&wrong_key_id, package.as_slice())
                .is_err()
        );
        let mut tampered_package = package.to_vec();
        let last_byte = tampered_package.len() - 1;
        tampered_package[last_byte] ^= 0x80;
        assert!(
            recipient
                .open_envelope_with_key_agreement(&key_id, &tampered_package)
                .is_err()
        );

        // Malformed, truncated and unsupported-format inputs must fail closed.
        assert!(
            recipient
                .open_envelope_with_key_agreement(&key_id, b"not an NQX package")
                .is_err()
        );
        assert!(
            recipient
                .open_envelope_with_key_agreement(&key_id, &package[..package.len() / 2])
                .is_err()
        );
        let mut unsupported_envelope: nexusq_core::vault::Envelope =
            nexusq_core::vault::from_slice(package.as_slice()).unwrap();
        unsupported_envelope.header.version =
            nexusq_core::vault::ENVELOPE_FORMAT_VERSION.saturating_add(1);
        let unsupported_format = nexusq_core::vault::to_vec(&unsupported_envelope).unwrap();
        assert!(
            recipient
                .open_envelope_with_key_agreement(&key_id, &unsupported_format)
                .is_err()
        );

        // A destination that is a directory must fail without replacing it.
        let blocked_destination = dir.path().join("destination-is-a-directory");
        fs::create_dir(&blocked_destination).unwrap();
        assert!(write_new_atomic(&blocked_destination, original).is_err());
        assert!(blocked_destination.is_dir());

        let empty_source = dir.path().join("empty-file.txt");
        let empty_package = dir.path().join("empty-file.nqx");
        fs::write(&empty_source, []).unwrap();
        create_shared_package_file(&empty_source, &empty_package, &card, || false).unwrap();
        let empty_envelope =
            read_bounded_file(&empty_package, MAX_SHARED_PACKAGE_BYTES, "shared package").unwrap();
        let recovered_empty = recipient
            .open_envelope_with_key_agreement(&key_id, empty_envelope.as_slice())
            .unwrap();
        assert!(recovered_empty.is_empty());

        let backup_path = dir.path().join("recipient.nqb");
        backup::export(&recipient, &backup_path, b"separate-backup-passphrase").unwrap();
        let restored_path = dir.path().join("recipient-restored.nqv");
        let restored_vault = backup::import(
            &backup_path,
            b"separate-backup-passphrase",
            &restored_path,
            b"new-recipient-password",
        )
        .unwrap();
        let restored = restored_vault.unlock(b"new-recipient-password").unwrap();
        let restored_identity = identity_id;
        assert_eq!(
            restored
                .find_identity(&restored_identity)
                .unwrap()
                .key_agreement_key
                .as_ref(),
            Some(&key_id)
        );
        let recovered_after_restore = restored
            .open_envelope_with_key_agreement(&key_id, &package)
            .unwrap();
        assert_eq!(recovered_after_restore.as_slice(), original);
    }

    #[test]
    fn shared_file_limit_rejects_files_larger_than_32_mib() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("oversized.bin");
        let file = File::create(&path).unwrap();
        file.set_len(MAX_SHARED_FILE_BYTES + 1).unwrap();
        assert!(read_bounded_file(&path, MAX_SHARED_FILE_BYTES, "source file").is_err());
    }

    #[test]
    fn empty_and_unicode_named_inputs_are_supported() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("résumé_日本語.txt");
        fs::write(&source, []).unwrap();
        let bytes = read_bounded_file(&source, MAX_SHARED_FILE_BYTES, "source file").unwrap();
        assert!(bytes.is_empty());
        let destination = dir.path().join("recuperado_ñ.txt");
        write_new_atomic(&destination, bytes.as_slice()).unwrap();
        assert_eq!(fs::read(destination).unwrap(), b"");
    }

    #[test]
    fn cancelling_package_creation_never_changes_the_original_or_publishes_output() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source file.txt");
        let output = dir.path().join("share.nqx");
        let original = b"original source bytes";
        fs::write(&source, original).unwrap();
        let card = sample_card();
        let mut checkpoints = 0;
        let result = create_shared_package_file(&source, &output, &card, || {
            checkpoints += 1;
            checkpoints == 3
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&source).unwrap(), original);
        assert!(!output.exists());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn atomic_writer_never_overwrites_an_existing_file() {
        let dir = TempDir::new().unwrap();
        let destination = dir.path().join("package.nqx");
        fs::write(&destination, b"original").unwrap();
        assert!(write_new_atomic(&destination, b"replacement").is_err());
        assert_eq!(fs::read(&destination).unwrap(), b"original");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
