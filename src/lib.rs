//! NEXUS-Q Vault integration spike.
//! All cryptography and vault file handling are delegated to nexusq-core.
pub mod folder_manifest;
use nexusq_core::storage::backup;
use nexusq_core::vault::{Algorithm, Purpose, Vault, VaultError};
use std::path::{Path, PathBuf};

/// Create a local vault, encrypt/decrypt a file, export an encrypted backup,
/// restore it into a new vault and decrypt the same envelope from that vault.
pub fn run_roundtrip(
    work_dir: &Path,
    input_file: &Path,
    vault_password: &[u8],
    backup_passphrase: &[u8],
) -> Result<RoundtripArtifacts, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(work_dir)?;
    let vault_path = work_dir.join("spike.nqv");
    let backup_path = work_dir.join("spike.nqb");
    let restored_file = work_dir.join("restored.bin");
    let backup_restored_file = work_dir.join("backup-restored.bin");
    let restored_vault_path = work_dir.join("restored.nqv");

    let vault = Vault::create(
        &vault_path,
        vault_password,
        Some("NEXUS-Q Vault spike".into()),
    )?;
    let mut session = vault.unlock(vault_password)?;
    let key_id = session.generate_key(Algorithm::Aes256Gcm, Purpose::Encrypt)?;
    session.activate_key(&key_id)?;
    let encrypted_file = session.encrypt_file(input_file, &key_id, Vec::new())?;
    session.decrypt_file(&encrypted_file, &restored_file)?;
    backup::export(&session, &backup_path, backup_passphrase)?;
    session.lock()?;

    let restored_vault = backup::import(
        &backup_path,
        backup_passphrase,
        &restored_vault_path,
        vault_password,
    )?;
    let restored_session = restored_vault.unlock(vault_password)?;
    // The restored vault contains the original key record and can open the
    // file envelope created before backup. This is the recovery proof.
    restored_session.decrypt_file(&encrypted_file, &backup_restored_file)?;
    restored_session.lock()?;

    Ok(RoundtripArtifacts {
        vault_path,
        backup_path,
        encrypted_file,
        restored_file,
        backup_restored_file,
        restored_vault_path,
    })
}

#[derive(Debug, Clone)]
pub struct RoundtripArtifacts {
    pub vault_path: PathBuf,
    pub backup_path: PathBuf,
    pub encrypted_file: PathBuf,
    pub restored_file: PathBuf,
    pub backup_restored_file: PathBuf,
    pub restored_vault_path: PathBuf,
}

pub type CoreVaultResult<T> = Result<T, VaultError>;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn file_roundtrip_and_backup_restore_roundtrip() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("private-document.txt");
        let work = dir.path().join("work");
        let original = b"NEXUS-Q Vault integration test: binary-safe plaintext.\0\x01";
        std::fs::write(&input, original).unwrap();

        let artifacts = run_roundtrip(
            &work,
            &input,
            b"test-only-vault-password",
            b"test-only-backup-passphrase",
        )
        .unwrap();

        assert!(artifacts.vault_path.exists());
        assert!(artifacts.backup_path.exists());
        assert!(artifacts.encrypted_file.exists());
        assert_eq!(std::fs::read(&artifacts.restored_file).unwrap(), original);
        assert_eq!(
            std::fs::read(&artifacts.backup_restored_file).unwrap(),
            original
        );
        // Encryption must preserve the source by default.
        assert_eq!(std::fs::read(&input).unwrap(), original);
    }

    #[test]
    fn wrong_vault_password_is_rejected() {
        let dir = tempdir().unwrap();
        let vault_path = dir.path().join("wrong-password.nqv");
        let password = b"correct-password";
        Vault::create(&vault_path, password, Some("password test".into())).unwrap();

        let opened = Vault::open(&vault_path).unwrap();
        let result = opened.unlock(b"incorrect-password");

        assert!(matches!(result, Err(VaultError::WrongPassword)));
    }

    #[test]
    fn backup_rejects_wrong_passphrase() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("source.bin");
        let work = dir.path().join("work");
        std::fs::write(&input, b"backup authentication test").unwrap();

        let artifacts = run_roundtrip(
            &work,
            &input,
            b"test-only-vault-password",
            b"correct-backup-passphrase",
        )
        .unwrap();

        let imported_path = work.join("must-not-import.nqv");
        let result = backup::import(
            &artifacts.backup_path,
            b"incorrect-backup-passphrase",
            &imported_path,
            b"test-only-vault-password",
        );

        assert!(result.is_err());
        assert!(!imported_path.exists());
    }

    #[test]
    fn corrupted_vault_body_is_rejected() {
        let dir = tempdir().unwrap();
        let vault_path = dir.path().join("corrupted.nqv");
        Vault::create(
            &vault_path,
            b"test-only-vault-password",
            Some("corruption test".into()),
        )
        .unwrap();

        let mut bytes = std::fs::read(&vault_path).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;
        std::fs::write(&vault_path, bytes).unwrap();

        let opened = Vault::open(&vault_path).unwrap();
        assert!(opened.unlock(b"test-only-vault-password").is_err());
    }

    #[test]
    fn tampered_envelope_is_rejected_without_plaintext_output() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("source.bin");
        let work = dir.path().join("work");
        let original = b"authenticated payload with binary bytes\0\xff";
        std::fs::write(&input, original).unwrap();

        let artifacts = run_roundtrip(
            &work,
            &input,
            b"test-only-vault-password",
            b"test-only-backup-passphrase",
        )
        .unwrap();

        let mut corrupted = std::fs::read(&artifacts.encrypted_file).unwrap();
        let last = corrupted.len() - 1;
        corrupted[last] ^= 0x80;
        let tampered_path = work.join("tampered.nqx");
        std::fs::write(&tampered_path, corrupted).unwrap();

        let vault = Vault::open(&artifacts.vault_path).unwrap();
        let session = vault.unlock(b"test-only-vault-password").unwrap();
        let plaintext_output = work.join("must-not-exist.bin");

        let result = session.decrypt_file(&tampered_path, &plaintext_output);
        assert!(matches!(result, Err(VaultError::Envelope(_))));
        assert!(!plaintext_output.exists());
        session.lock().unwrap();
    }

    #[test]
    fn backup_recovers_vault_with_a_new_password() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("recovery-source.bin");
        let work = dir.path().join("recovery-work");
        let original = b"backup recovery with independent credentials\0\xff";
        std::fs::write(&input, original).unwrap();

        let artifacts = run_roundtrip(
            &work,
            &input,
            b"original-vault-password",
            b"independent-backup-passphrase",
        )
        .unwrap();

        let recovered_path = work.join("recovered-with-new-password.nqv");
        let recovered = backup::import(
            &artifacts.backup_path,
            b"independent-backup-passphrase",
            &recovered_path,
            b"new-vault-password",
        )
        .unwrap();

        assert!(matches!(
            recovered.unlock(b"original-vault-password"),
            Err(VaultError::WrongPassword)
        ));

        let session = recovered.unlock(b"new-vault-password").unwrap();
        let restored_file = work.join("recovered-plaintext.bin");
        session
            .decrypt_file(&artifacts.encrypted_file, &restored_file)
            .unwrap();
        assert_eq!(std::fs::read(restored_file).unwrap(), original);
        session.lock().unwrap();
    }
}
