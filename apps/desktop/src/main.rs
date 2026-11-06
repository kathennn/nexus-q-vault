#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use nexusq_core::{
    storage::backup,
    vault::{Algorithm, KeyStatus, Purpose, Session, Vault},
};
use std::sync::Mutex;
use tauri::State;
use zeroize::Zeroizing;

#[derive(Default)]
struct AppState {
    session: Mutex<Option<Session>>,
}

fn secret_bytes(value: String) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(value.into_bytes())
}

fn active_session<'a>(
    state: &'a State<'_, AppState>,
) -> Result<std::sync::MutexGuard<'a, Option<Session>>, String> {
    let guard = state
        .session
        .lock()
        .map_err(|_| "El estado seguro de la aplicación no está disponible".to_string())?;
    if guard.is_none() {
        return Err("Primero debes desbloquear un vault".into());
    }
    Ok(guard)
}

#[tauri::command]
fn vault_status(state: State<'_, AppState>) -> Result<bool, String> {
    let guard = state
        .session
        .lock()
        .map_err(|_| "El estado seguro de la aplicación no está disponible".to_string())?;
    Ok(guard.is_some())
}

#[tauri::command]
fn create_vault(
    state: State<'_, AppState>,
    path: String,
    password: String,
) -> Result<String, String> {
    if path.trim().is_empty() {
        return Err("Indica dónde guardar el vault".into());
    }
    if password.len() < 12 {
        return Err("Usa una contraseña de al menos 12 bytes".into());
    }
    let mut guard = state
        .session
        .lock()
        .map_err(|_| "El estado seguro de la aplicación no está disponible".to_string())?;
    if guard.is_some() {
        return Err("Bloquea el vault actual antes de crear otro".into());
    }

    let password = secret_bytes(password);
    let vault = Vault::create(&path, password.as_slice(), Some("NEXUS-Q Vault".into()))
        .map_err(|e| format!("No se pudo crear el vault: {e}"))?;
    let mut session = vault
        .unlock(password.as_slice())
        .map_err(|e| format!("No se pudo inicializar el vault: {e}"))?;
    let key_id = session
        .generate_key(Algorithm::Aes256Gcm, Purpose::Encrypt)
        .map_err(|e| format!("No se pudo preparar la clave de archivos: {e}"))?;
    session
        .activate_key(&key_id)
        .map_err(|e| format!("No se pudo activar la clave de archivos: {e}"))?;
    let vault = session
        .lock()
        .map_err(|e| format!("No se pudo guardar el vault: {e}"))?;
    let session = vault
        .unlock(password.as_slice())
        .map_err(|e| format!("No se pudo verificar el vault creado: {e}"))?;
    *guard = Some(session);
    Ok("Vault creado, verificado y desbloqueado".into())
}

#[tauri::command]
fn open_vault(
    state: State<'_, AppState>,
    path: String,
    password: String,
) -> Result<String, String> {
    if path.trim().is_empty() {
        return Err("Indica la ruta del vault".into());
    }
    let mut guard = state
        .session
        .lock()
        .map_err(|_| "El estado seguro de la aplicación no está disponible".to_string())?;
    if guard.is_some() {
        return Err("Bloquea el vault actual antes de abrir otro".into());
    }
    let password = secret_bytes(password);
    let session = Vault::open(&path)
        .and_then(|vault| vault.unlock(password.as_slice()))
        .map_err(|e| format!("No se pudo desbloquear el vault: {e}"))?;
    *guard = Some(session);
    Ok("Vault desbloqueado".into())
}

#[tauri::command]
fn lock_vault(state: State<'_, AppState>) -> Result<String, String> {
    let mut guard = state
        .session
        .lock()
        .map_err(|_| "El estado seguro de la aplicación no está disponible".to_string())?;
    let session = guard
        .take()
        .ok_or_else(|| "No hay ningún vault desbloqueado".to_string())?;
    session
        .lock()
        .map_err(|e| format!("No se pudo guardar y bloquear el vault: {e}"))?;
    Ok("Vault guardado y bloqueado".into())
}

#[tauri::command]
fn encrypt_file(state: State<'_, AppState>, input_path: String) -> Result<String, String> {
    let guard = active_session(&state)?;
    let session = guard.as_ref().expect("active_session checked");
    let key_id = session
        .list_keys()
        .find(|record| record.purpose() == Purpose::Encrypt && record.status() == KeyStatus::Active)
        .map(|record| record.key_id().clone())
        .ok_or_else(|| "El vault no tiene una clave activa para cifrar".to_string())?;
    let output = session
        .encrypt_file(&input_path, &key_id, Vec::new())
        .map_err(|e| format!("No se pudo cifrar el archivo: {e}"))?;
    Ok(format!("Archivo cifrado: {}", output.display()))
}

#[tauri::command]
fn decrypt_file(
    state: State<'_, AppState>,
    input_path: String,
    output_path: String,
) -> Result<String, String> {
    if output_path.trim().is_empty() {
        return Err("Indica dónde restaurar el archivo".into());
    }
    let guard = active_session(&state)?;
    let session = guard.as_ref().expect("active_session checked");
    session
        .decrypt_file(&input_path, &output_path)
        .map_err(|e| format!("No se pudo restaurar el archivo: {e}"))?;
    Ok(format!("Archivo restaurado: {output_path}"))
}

#[tauri::command]
fn export_backup(
    state: State<'_, AppState>,
    backup_path: String,
    backup_passphrase: String,
) -> Result<String, String> {
    if backup_path.trim().is_empty() {
        return Err("Indica dónde guardar el backup".into());
    }
    if backup_passphrase.len() < 12 {
        return Err("Usa una contraseña de backup de al menos 12 bytes".into());
    }
    let guard = active_session(&state)?;
    let session = guard.as_ref().expect("active_session checked");
    let passphrase = secret_bytes(backup_passphrase);
    backup::export(session, &backup_path, passphrase.as_slice())
        .map_err(|e| format!("No se pudo crear el backup: {e}"))?;
    Ok("Backup cifrado creado".into())
}

#[tauri::command]
fn import_backup(
    state: State<'_, AppState>,
    backup_path: String,
    backup_passphrase: String,
    new_vault_path: String,
    new_vault_password: String,
) -> Result<String, String> {
    if backup_path.trim().is_empty() || new_vault_path.trim().is_empty() {
        return Err("Indica la ruta del backup y la del nuevo vault".into());
    }
    if backup_passphrase.len() < 12 {
        return Err("Usa la contraseña independiente del backup (mínimo 12 bytes)".into());
    }
    if new_vault_password.len() < 12 {
        return Err("Usa una contraseña nueva de al menos 12 bytes".into());
    }
    let mut guard = state
        .session
        .lock()
        .map_err(|_| "El estado seguro de la aplicación no está disponible".to_string())?;
    if guard.is_some() {
        return Err("Bloquea el vault actual antes de importar un backup".into());
    }
    let passphrase = secret_bytes(backup_passphrase);
    let password = secret_bytes(new_vault_password);
    let vault = backup::import(
        &backup_path,
        passphrase.as_slice(),
        &new_vault_path,
        password.as_slice(),
    )
    .map_err(|e| format!("No se pudo restaurar el backup: {e}"))?;
    let session = vault
        .unlock(password.as_slice())
        .map_err(|e| format!("No se pudo verificar el vault restaurado: {e}"))?;
    *guard = Some(session);
    Ok("Backup restaurado en un nuevo vault y verificado".into())
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            vault_status,
            create_vault,
            open_vault,
            lock_vault,
            encrypt_file,
            decrypt_file,
            export_backup,
            import_backup
        ])
        .run(tauri::generate_context!())
        .expect("error while running NEXUS-Q Vault");
}
