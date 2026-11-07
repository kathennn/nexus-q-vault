//! Exercises backup recovery across two fresh OS processes and isolated profiles.
use nexus_q_vault_spike::folder_manifest::{decrypt_folder, encrypt_folder};
use nexusq_core::storage::backup;
use nexusq_core::vault::{Algorithm, Purpose, Vault, VaultError};
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

const CHILD_STAGE: &str = "NEXUS_Q_RECOVERY_CHILD_STAGE";
const RECOVERY_ROOT: &str = "NEXUS_Q_RECOVERY_ROOT";
const VAULT_PASSWORD: &[u8] = b"test-only-original-vault-password";
const BACKUP_PASSPHRASE: &[u8] = b"test-only-independent-backup-passphrase";
const RECOVERED_PASSWORD: &[u8] = b"test-only-new-vault-password";

const BINARY_CONTENT: &[u8] = b"independent recovery binary payload\0\x01\xfe\xff";
const FOLDER_CONTENT: &[u8] = b"independent recovery nested folder payload";

#[test]
fn recovery_works_across_independent_processes() {
    if let Ok(stage) = env::var(CHILD_STAGE) {
        let root = PathBuf::from(env::var_os(RECOVERY_ROOT).expect("recovery root is set"));
        let result = match stage.as_str() {
            "create" => create_source_installation(&root),
            "restore" => restore_in_independent_installation(&root),
            _ => panic!("unknown recovery child stage: {stage}"),
        };
        result.unwrap();
        return;
    }

    let temporary = tempdir().unwrap();
    let root = temporary.path();
    let profile_a = root.join("profiles/installation-a");
    let profile_b = root.join("profiles/installation-b");
    fs::create_dir_all(&profile_a).unwrap();
    fs::create_dir_all(&profile_b).unwrap();

    run_child("create", root, &profile_a);
    let original_vault = fs::read(root.join("installation-a/vault.nqv")).unwrap();
    let original_binary = fs::read(root.join("installation-a/source-files/binary.dat")).unwrap();
    let original_folder_file =
        fs::read(root.join("installation-a/source-folder/Nested/Δοκιμή/画像.txt")).unwrap();

    run_child("restore", root, &profile_b);

    // The recovery process must not alter or replace the source installation.
    assert_eq!(
        fs::read(root.join("installation-a/vault.nqv")).unwrap(),
        original_vault
    );
    assert_eq!(
        fs::read(root.join("installation-a/source-files/binary.dat")).unwrap(),
        original_binary
    );
    assert_eq!(
        fs::read(root.join("installation-a/source-folder/Nested/Δοκιμή/画像.txt")).unwrap(),
        original_folder_file
    );
    assert!(root.join("installation-b/recovered.nqv").is_file());
}

fn run_child(stage: &str, root: &Path, profile: &Path) {
    let temporary = profile.join("temporary");
    let config = profile.join("config");
    let data = profile.join("data");
    let cache = profile.join("cache");
    let local_data = profile.join("local-data");
    let runtime = profile.join("runtime");
    for directory in [&temporary, &config, &data, &cache, &local_data, &runtime] {
        fs::create_dir_all(directory).unwrap();
    }

    let status = Command::new(env::current_exe().expect("test executable path"))
        .current_dir(profile)
        .arg("--exact")
        .arg("recovery_works_across_independent_processes")
        .arg("--nocapture")
        .env(CHILD_STAGE, stage)
        .env(RECOVERY_ROOT, root)
        // Keep each child process on a distinct, empty user/application profile.
        .env("HOME", profile)
        .env("USERPROFILE", profile)
        .env("APPDATA", &config)
        .env("LOCALAPPDATA", local_data)
        .env("XDG_CONFIG_HOME", &config)
        .env("XDG_DATA_HOME", data)
        .env("XDG_CACHE_HOME", cache)
        .env("XDG_RUNTIME_DIR", runtime)
        .env("TMPDIR", &temporary)
        .env("TMP", &temporary)
        .env("TEMP", &temporary)
        .status()
        .expect("start isolated recovery process");

    assert!(
        status.success(),
        "recovery child process {stage:?} failed: {status}"
    );
}

fn create_source_installation(root: &Path) -> Result<(), Box<dyn Error + Send + Sync>> {
    let installation = root.join("installation-a");
    let source_files = installation.join("source-files");
    let source_folder = installation.join("source-folder");
    let vault_path = installation.join("vault.nqv");
    let backup_path = installation.join("vault-backup.nqb");
    let folder_vault_path = installation.join("protected-folder");
    let package = root.join("recovery-package");
    let packaged_files = package.join("payloads/files");
    let packaged_folder = package.join("payloads/folder-vault");

    fs::create_dir_all(&source_files)?;
    fs::create_dir_all(source_folder.join("Nested/Δοκιμή"))?;
    fs::create_dir_all(&packaged_files)?;

    fs::write(source_files.join("binary.dat"), BINARY_CONTENT)?;
    fs::write(source_files.join("empty.txt"), b"")?;
    fs::write(
        source_files.join("résumé-東京.txt"),
        b"non-ASCII filename payload",
    )?;
    fs::write(source_folder.join("Nested/Δοκιμή/画像.txt"), FOLDER_CONTENT)?;
    fs::write(source_folder.join("empty-folder-file.bin"), b"")?;

    let vault = Vault::create(
        &vault_path,
        VAULT_PASSWORD,
        Some("independent recovery source".into()),
    )?;
    let mut session = vault.unlock(VAULT_PASSWORD)?;
    let key_id = session.generate_key(Algorithm::Aes256Gcm, Purpose::Encrypt)?;
    session.activate_key(&key_id)?;

    for name in ["binary.dat", "empty.txt", "résumé-東京.txt"] {
        let source = source_files.join(name);
        let encrypted = session.encrypt_file(&source, &key_id, Vec::new())?;
        let destination = packaged_files.join(format!("{name}.nqx"));
        fs::copy(encrypted, destination)?;
    }

    encrypt_folder(&session, &key_id, &source_folder, &folder_vault_path)?;
    backup::export(&session, &backup_path, BACKUP_PASSPHRASE)?;
    session.lock()?;

    // The .nqb contains vault/key state only; document payloads travel separately.
    let backup_bytes = fs::read(&backup_path)?;
    for marker in [
        b"independent recovery binary payload".as_slice(),
        b"non-ASCII filename payload".as_slice(),
        b"independent recovery nested folder payload".as_slice(),
        "résumé-東京.txt".as_bytes(),
        "画像.txt".as_bytes(),
    ] {
        assert!(
            !contains_bytes(&backup_bytes, marker),
            "backup unexpectedly contains document data"
        );
    }

    for entry in fs::read_dir(&packaged_files)? {
        let envelope = fs::read(entry?.path())?;
        assert!(
            !contains_bytes(&backup_bytes, &envelope),
            "backup unexpectedly contains an individual-file payload"
        );
    }
    for entry in fs::read_dir(folder_vault_path.join("objects"))? {
        let envelope = fs::read(entry?.path())?;
        assert!(
            !contains_bytes(&backup_bytes, &envelope),
            "backup unexpectedly contains a folder-object payload"
        );
    }

    fs::copy(&backup_path, package.join("vault-backup.nqb"))?;
    copy_directory(&folder_vault_path, &packaged_folder)?;
    Ok(())
}

fn restore_in_independent_installation(root: &Path) -> Result<(), Box<dyn Error + Send + Sync>> {
    let installation = root.join("installation-b");
    fs::create_dir_all(&installation)?;
    let package = root.join("recovery-package");
    let backup_path = package.join("vault-backup.nqb");
    let restored_vault_path = installation.join("recovered.nqv");

    // Wrong credentials and a damaged backup must fail without publishing a vault.
    let wrong_password_destination = installation.join("wrong-password.nqv");
    assert!(
        backup::import(
            &backup_path,
            b"incorrect-independent-backup-passphrase",
            &wrong_password_destination,
            RECOVERED_PASSWORD,
        )
        .is_err()
    );
    assert!(!wrong_password_destination.exists());

    let missing_backup = installation.join("missing.nqb");
    let missing_backup_destination = installation.join("missing-backup.nqv");
    assert!(
        backup::import(
            &missing_backup,
            BACKUP_PASSPHRASE,
            &missing_backup_destination,
            RECOVERED_PASSWORD,
        )
        .is_err()
    );
    assert!(!missing_backup_destination.exists());

    let corrupted_backup = installation.join("corrupted.nqb");
    let mut corrupted_bytes = fs::read(&backup_path)?;
    let last_byte = corrupted_bytes
        .len()
        .checked_sub(1)
        .ok_or_else(|| std::io::Error::other("backup unexpectedly empty"))?;
    corrupted_bytes[last_byte] ^= 0x80;
    fs::write(&corrupted_backup, corrupted_bytes)?;
    let corrupted_destination = installation.join("corrupted-import.nqv");
    assert!(
        backup::import(
            &corrupted_backup,
            BACKUP_PASSPHRASE,
            &corrupted_destination,
            RECOVERED_PASSWORD,
        )
        .is_err()
    );
    assert!(!corrupted_destination.exists());

    // Truncated headers and encrypted bodies must fail without creating a vault.
    let truncated_bytes = fs::read(&backup_path)?;
    let truncated_header = installation.join("truncated-header.nqb");
    fs::write(&truncated_header, &truncated_bytes[..8])?;
    let truncated_header_destination = installation.join("truncated-header.nqv");
    assert!(
        backup::import(
            &truncated_header,
            BACKUP_PASSPHRASE,
            &truncated_header_destination,
            RECOVERED_PASSWORD,
        )
        .is_err()
    );
    assert!(!truncated_header_destination.exists());

    let header_len = u32::from_be_bytes(
        truncated_bytes[4..8]
            .try_into()
            .expect("backup header length prefix"),
    ) as usize;
    let body_start = 8 + header_len;
    let truncated_body = installation.join("truncated-body.nqb");
    fs::write(&truncated_body, &truncated_bytes[..body_start + 12 + 8])?;
    let truncated_body_destination = installation.join("truncated-body.nqv");
    assert!(
        backup::import(
            &truncated_body,
            BACKUP_PASSPHRASE,
            &truncated_body_destination,
            RECOVERED_PASSWORD,
        )
        .is_err()
    );
    assert!(!truncated_body_destination.exists());

    // Import must not overwrite or modify an existing destination.
    let existing_destination = installation.join("existing-destination.nqv");
    let existing_contents = b"preserve pre-existing destination";
    fs::write(&existing_destination, existing_contents)?;
    assert!(
        backup::import(
            &backup_path,
            BACKUP_PASSPHRASE,
            &existing_destination,
            RECOVERED_PASSWORD,
        )
        .is_err()
    );
    assert_eq!(fs::read(&existing_destination)?, existing_contents);

    let restored_vault = backup::import(
        &backup_path,
        BACKUP_PASSPHRASE,
        &restored_vault_path,
        RECOVERED_PASSWORD,
    )?;
    assert!(matches!(
        restored_vault.unlock(VAULT_PASSWORD),
        Err(VaultError::WrongPassword)
    ));
    let restored_session = restored_vault.unlock(RECOVERED_PASSWORD)?;

    let packaged_files = package.join("payloads/files");
    let restored_files = installation.join("restored-files");
    fs::create_dir_all(&restored_files)?;
    for (name, expected) in [
        ("binary.dat", BINARY_CONTENT),
        ("empty.txt", b"".as_slice()),
        ("résumé-東京.txt", b"non-ASCII filename payload".as_slice()),
    ] {
        let envelope = packaged_files.join(format!("{name}.nqx"));
        let output = restored_files.join(name);
        restored_session.decrypt_file(&envelope, &output)?;
        assert_eq!(fs::read(output)?, expected, "restored file differs: {name}");
    }

    // A modified payload must fail closed, leaving no plaintext output.
    let tampered_envelope = installation.join("tampered.nqx");
    let original_envelope = packaged_files.join("binary.dat.nqx");
    let mut envelope_bytes = fs::read(&original_envelope)?;
    let last_byte = envelope_bytes
        .len()
        .checked_sub(1)
        .ok_or_else(|| std::io::Error::other("encrypted payload unexpectedly empty"))?;
    envelope_bytes[last_byte] ^= 0x01;
    fs::write(&tampered_envelope, envelope_bytes)?;
    let forbidden_output = installation.join("must-not-exist.bin");
    assert!(
        restored_session
            .decrypt_file(&tampered_envelope, &forbidden_output)
            .is_err()
    );
    assert!(!forbidden_output.exists());

    // A missing encrypted file payload must fail without producing plaintext.
    let missing_file_output = installation.join("missing-file-output.bin");
    assert!(
        restored_session
            .decrypt_file(
                installation.join("missing-payload.nqx"),
                &missing_file_output,
            )
            .is_err()
    );
    assert!(!missing_file_output.exists());

    let tampered_folder = installation.join("tampered-folder-vault");
    copy_directory(&package.join("payloads/folder-vault"), &tampered_folder)?;
    let object = fs::read_dir(tampered_folder.join("objects"))?
        .next()
        .ok_or_else(|| std::io::Error::other("folder vault has no encrypted objects"))??
        .path();
    let mut object_bytes = fs::read(&object)?;
    let last_byte = object_bytes
        .len()
        .checked_sub(1)
        .ok_or_else(|| std::io::Error::other("folder object unexpectedly empty"))?;
    object_bytes[last_byte] ^= 0x01;
    fs::write(object, object_bytes)?;
    let failed_folder_restore = installation.join("must-not-restore-folder");
    assert!(decrypt_folder(&restored_session, &tampered_folder, &failed_folder_restore,).is_err());
    assert!(!failed_folder_restore.exists());

    // A missing folder object must also fail closed without publishing a folder.
    let missing_object_folder = installation.join("missing-object-folder-vault");
    copy_directory(
        &package.join("payloads/folder-vault"),
        &missing_object_folder,
    )?;
    let missing_object = fs::read_dir(missing_object_folder.join("objects"))?
        .next()
        .ok_or_else(|| std::io::Error::other("folder vault has no encrypted objects"))??
        .path();
    fs::remove_file(missing_object)?;
    let failed_missing_folder = installation.join("must-not-restore-missing-folder");
    assert!(
        decrypt_folder(
            &restored_session,
            &missing_object_folder,
            &failed_missing_folder,
        )
        .is_err()
    );
    assert!(!failed_missing_folder.exists());

    let restored_folder = installation.join("restored-folder");
    let restored_count = decrypt_folder(
        &restored_session,
        &package.join("payloads/folder-vault"),
        &restored_folder,
    )?;
    assert_eq!(restored_count, 2);
    assert_eq!(
        fs::read(restored_folder.join("Nested/Δοκιμή/画像.txt"))?,
        FOLDER_CONTENT
    );
    assert_eq!(
        fs::read(restored_folder.join("empty-folder-file.bin"))?,
        b""
    );

    restored_session.lock()?;
    Ok(())
}

fn copy_directory(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_directory(&source_path, &destination_path)?;
        } else {
            fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}
