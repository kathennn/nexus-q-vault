//! NEXUS-Q Vault integration spike.
//! All cryptography and vault file handling are delegated to nexusq-core.
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
}
