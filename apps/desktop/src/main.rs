#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use nexus_q_vault_spike::folder_manifest::{
    decrypt_folder as decrypt_folder_from_vault, encrypt_folder as encrypt_folder_to_vault,
};
use nexusq_core::{
    storage::backup,
    vault::{Algorithm, KeyId, KeyStatus, Purpose, Session, Vault},
};
use std::{
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State};
use tauri_plugin_dialog::{DialogExt, FilePath};
use zeroize::Zeroizing;

struct AppState {
    session: Mutex<Option<Session>>,
    last_activity: Mutex<Instant>,
    idle_timeout: Mutex<Duration>,
    auto_lock_retry_after: Mutex<Option<Instant>>,
    watchdog_stop: AtomicBool,
}
impl Default for AppState {
    fn default() -> Self {
        Self {
            session: Mutex::new(None),
            last_activity: Mutex::new(Instant::now()),
            idle_timeout: Mutex::new(Duration::from_secs(300)),
            auto_lock_retry_after: Mutex::new(None),
            watchdog_stop: AtomicBool::new(false),
        }
    }
}
const AUTO_LOCK_RETRY_DELAY: Duration = Duration::from_secs(30);

fn idle_timeout_from_seconds(seconds: u64) -> Result<Duration, String> {
    if seconds != 0 && ![60, 300, 600, 900, 1800].contains(&seconds) {
        return Err("El tiempo de inactividad seleccionado no es válido".into());
    }
    Ok(Duration::from_secs(seconds))
}

fn inactivity_elapsed(last_activity: Instant, now: Instant, timeout: Duration) -> bool {
    !timeout.is_zero() && now.duration_since(last_activity) >= timeout
}

fn touch_activity(state: &AppState) -> Result<(), String> {
    *state
        .last_activity
        .lock()
        .map_err(|_| "No se pudo actualizar la actividad".to_string())? = Instant::now();
    *state
        .auto_lock_retry_after
        .lock()
        .map_err(|_| "No se pudo actualizar el estado de bloqueo".to_string())? = None;
    Ok(())
}

fn defer_auto_lock_retry(state: &AppState) {
    if let Ok(mut retry_after) = state.auto_lock_retry_after.lock() {
        *retry_after = Some(Instant::now() + AUTO_LOCK_RETRY_DELAY);
    }
}
fn lock_session_slot(slot: &mut Option<Session>) -> Result<bool, String> {
    let Some(session) = slot.take() else {
        return Ok(false);
    };
    match session.lock_recoverable() {
        Ok(_) => Ok(true),
        Err(failure) => {
            let (session, error) = failure.into_parts();
            *slot = Some(session);
            Err(format!("No se pudo guardar y bloquear el vault: {error}"))
        }
    }
}

fn lock_session(state: &AppState) -> Result<bool, String> {
    let mut guard = state
        .session
        .lock()
        .map_err(|_| "El estado seguro de la aplicación no está disponible".to_string())?;
    let result = lock_session_slot(&mut guard);
    if result.is_err() {
        defer_auto_lock_retry(state);
    }
    result
}
#[derive(Debug, PartialEq, Eq)]
enum AutoLockOutcome {
    NotDue,
    Locked,
    Failed(String),
}

fn auto_lock_state(state: &AppState) -> AutoLockOutcome {
    let now = Instant::now();
    let retry_after = match state.auto_lock_retry_after.lock() {
        Ok(value) => *value,
        Err(_) => return AutoLockOutcome::NotDue,
    };
    if retry_after.is_some_and(|deadline| now < deadline) {
        return AutoLockOutcome::NotDue;
    }

    let timeout = match state.idle_timeout.lock() {
        Ok(value) => *value,
        Err(_) => return AutoLockOutcome::NotDue,
    };
    let last_activity = match state.last_activity.lock() {
        Ok(value) => *value,
        Err(_) => return AutoLockOutcome::NotDue,
    };
    if !inactivity_elapsed(last_activity, now, timeout) {
        return AutoLockOutcome::NotDue;
    }

    let mut session = match state.session.lock() {
        Ok(value) => value,
        Err(_) => return AutoLockOutcome::NotDue,
    };
    let still_idle = state
        .last_activity
        .lock()
        .map(|last| inactivity_elapsed(*last, Instant::now(), timeout))
        .unwrap_or(false);
    if !still_idle || session.is_none() {
        return AutoLockOutcome::NotDue;
    }

    match lock_session_slot(&mut session) {
        Ok(true) => {
            if let Ok(mut retry_after) = state.auto_lock_retry_after.lock() {
                *retry_after = None;
            }
            AutoLockOutcome::Locked
        }
        Ok(false) => AutoLockOutcome::NotDue,
        Err(error) => {
            defer_auto_lock_retry(state);
            AutoLockOutcome::Failed(error)
        }
    }
}

fn auto_lock_if_idle(app: &AppHandle) {
    let state = app.state::<AppState>();
    match auto_lock_state(&state) {
        AutoLockOutcome::NotDue => {}
        AutoLockOutcome::Locked => {
            let _ = app.emit("vault-auto-locked", "inactivity");
        }
        AutoLockOutcome::Failed(error) => {
            let _ = app.emit(
                "vault-lock-failed",
                format!("No se pudo guardar y bloquear automáticamente el vault: {error}"),
            );
        }
    }
}

fn secret_bytes(value: String) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(value.into_bytes())
}

fn confirmed_secret(
    value: String,
    confirmation: String,
    field_name: &str,
) -> Result<Zeroizing<Vec<u8>>, String> {
    let value = secret_bytes(value);
    let confirmation = secret_bytes(confirmation);
    if value.is_empty() {
        return Err(format!("{field_name} no puede estar vacía"));
    }
    if value.as_slice() != confirmation.as_slice() {
        return Err(format!("{field_name} no coincide con su confirmación"));
    }
    Ok(value)
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

fn active_file_key(session: &Session) -> Result<KeyId, String> {
    session
        .list_keys()
        .find(|record| record.purpose() == Purpose::Encrypt && record.status() == KeyStatus::Active)
        .map(|record| record.key_id().clone())
        .ok_or_else(|| "El vault no tiene una clave activa para cifrar".to_string())
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
fn record_activity(state: State<'_, AppState>) -> Result<(), String> {
    let _session = state
        .session
        .lock()
        .map_err(|_| "El estado seguro de la aplicación no está disponible".to_string())?;
    touch_activity(&state)
}
#[tauri::command]
fn set_idle_timeout(state: State<'_, AppState>, seconds: u64) -> Result<(), String> {
    let timeout = idle_timeout_from_seconds(seconds)?;
    let _session = state
        .session
        .lock()
        .map_err(|_| "El estado seguro de la aplicación no está disponible".to_string())?;
    *state
        .idle_timeout
        .lock()
        .map_err(|_| "No se pudo guardar el tiempo de bloqueo".to_string())? = timeout;
    touch_activity(&state)
}
#[tauri::command]
fn create_vault(
    state: State<'_, AppState>,
    path: String,
    password: String,
    password_confirmation: String,
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

    let password = confirmed_secret(password, password_confirmation, "La contraseña")?;
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
    touch_activity(&state)?;
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
    touch_activity(&state)?;
    Ok("Vault desbloqueado".into())
}

#[tauri::command]
fn lock_vault(state: State<'_, AppState>) -> Result<String, String> {
    if lock_session(&state)? {
        Ok("Vault guardado y bloqueado".into())
    } else {
        Err("No hay ningún vault desbloqueado".into())
    }
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
fn encrypt_folder(
    state: State<'_, AppState>,
    source_path: String,
    vault_folder_path: String,
) -> Result<String, String> {
    if source_path.trim().is_empty() || vault_folder_path.trim().is_empty() {
        return Err("Indica la carpeta original y dónde guardar el vault de carpeta".into());
    }
    let guard = active_session(&state)?;
    let session = guard.as_ref().expect("active_session checked");
    let key_id = active_file_key(session)?;
    let count = encrypt_folder_to_vault(
        session,
        &key_id,
        std::path::Path::new(&source_path),
        std::path::Path::new(&vault_folder_path),
    )
    .map_err(|error| format!("No se pudo cifrar la carpeta: {error}"))?;
    Ok(format!("Carpeta cifrada: {count} archivos protegidos"))
}

#[tauri::command]
fn decrypt_folder(
    state: State<'_, AppState>,
    vault_folder_path: String,
    destination_path: String,
) -> Result<String, String> {
    if vault_folder_path.trim().is_empty() || destination_path.trim().is_empty() {
        return Err("Indica la carpeta vault y el destino de restauración".into());
    }
    let guard = active_session(&state)?;
    let session = guard.as_ref().expect("active_session checked");
    let count = decrypt_folder_from_vault(
        session,
        std::path::Path::new(&vault_folder_path),
        std::path::Path::new(&destination_path),
    )
    .map_err(|error| format!("No se pudo restaurar la carpeta: {error}"))?;
    Ok(format!("Carpeta restaurada: {count} archivos recuperados"))
}

#[tauri::command]
fn export_backup(
    state: State<'_, AppState>,
    backup_path: String,
    backup_passphrase: String,
    backup_passphrase_confirmation: String,
) -> Result<String, String> {
    if backup_path.trim().is_empty() {
        return Err("Indica dónde guardar el backup".into());
    }
    if backup_passphrase.len() < 12 {
        return Err("Usa una contraseña de backup de al menos 12 bytes".into());
    }
    let guard = active_session(&state)?;
    let session = guard.as_ref().expect("active_session checked");
    let passphrase = confirmed_secret(
        backup_passphrase,
        backup_passphrase_confirmation,
        "La contraseña del backup",
    )?;
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
    new_vault_password_confirmation: String,
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
    let password = confirmed_secret(
        new_vault_password,
        new_vault_password_confirmation,
        "La nueva contraseña del vault",
    )?;
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
    touch_activity(&state)?;
    Ok("Backup restaurado en un nuevo vault y verificado".into())
}

fn selected_path(path: Option<FilePath>) -> Result<Option<String>, String> {
    match path {
        Some(path) => {
            let path = path
                .into_path()
                .map_err(|error| format!("No se pudo interpretar la ruta seleccionada: {error}"))?;
            let path = path
                .to_str()
                .ok_or_else(|| "La ruta seleccionada no es compatible con UTF-8".to_string())?;
            Ok(Some(path.to_owned()))
        }
        None => Ok(None),
    }
}

#[tauri::command]
async fn pick_file_path(app: AppHandle, kind: String) -> Result<Option<String>, String> {
    let selected = match kind.as_str() {
        "vault" => app
            .dialog()
            .file()
            .set_title("Seleccionar vault NEXUS-Q")
            .add_filter("Vault NEXUS-Q", &["nqv"])
            .blocking_pick_file(),
        "backup" => app
            .dialog()
            .file()
            .set_title("Seleccionar backup cifrado")
            .add_filter("Backup NEXUS-Q", &["nqb"])
            .blocking_pick_file(),
        "encrypted" => app
            .dialog()
            .file()
            .set_title("Seleccionar archivo cifrado")
            .add_filter("Archivo NEXUS-Q", &["nqx"])
            .blocking_pick_file(),
        "any" => app
            .dialog()
            .file()
            .set_title("Seleccionar archivo original")
            .blocking_pick_file(),
        _ => return Err("Tipo de selector de archivo no válido".into()),
    };
    selected_path(selected)
}

#[tauri::command]
async fn pick_save_path(app: AppHandle, kind: String) -> Result<Option<String>, String> {
    let selected = match kind.as_str() {
        "vault" => app
            .dialog()
            .file()
            .set_title("Guardar vault NEXUS-Q")
            .add_filter("Vault NEXUS-Q", &["nqv"])
            .set_file_name("nexus-q-vault.nqv")
            .blocking_save_file(),
        "backup" => app
            .dialog()
            .file()
            .set_title("Guardar backup cifrado")
            .add_filter("Backup NEXUS-Q", &["nqb"])
            .set_file_name("backup.nqb")
            .blocking_save_file(),
        "restored" => app
            .dialog()
            .file()
            .set_title("Guardar archivo restaurado")
            .set_file_name("restored-file")
            .blocking_save_file(),
        _ => return Err("Tipo de destino no válido".into()),
    };
    selected_path(selected)
}

#[tauri::command]
async fn pick_folder_path(app: AppHandle, kind: String) -> Result<Option<String>, String> {
    let title = match kind.as_str() {
        "source" => "Seleccionar carpeta original",
        "vault-folder" => "Seleccionar carpeta vault",
        "parent" => "Seleccionar carpeta contenedora",
        _ => return Err("Tipo de selector de carpeta no válido".into()),
    };
    let selected = app.dialog().file().set_title(title).blocking_pick_folder();
    selected_path(selected)
}

#[tauri::command]
fn open_project_repository() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("cmd")
        .args(["/C", "start", "", "https://github.com/lecodev-26/nexus-q"])
        .spawn();
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open")
        .arg("https://github.com/lecodev-26/nexus-q")
        .spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let result = std::process::Command::new("xdg-open")
        .arg("https://github.com/lecodev-26/nexus-q")
        .spawn();
    result
        .map(|_| ())
        .map_err(|error| format!("No se pudo abrir GitHub: {error}"))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|app| {
            let handle = app.handle().clone();
            thread::spawn(move || {
                loop {
                    thread::sleep(Duration::from_secs(1));
                    let should_stop = {
                        let state = handle.state::<AppState>();
                        state.watchdog_stop.load(Ordering::Relaxed)
                    };
                    if should_stop {
                        break;
                    }
                    auto_lock_if_idle(&handle);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            vault_status,
            record_activity,
            set_idle_timeout,
            create_vault,
            open_vault,
            lock_vault,
            encrypt_file,
            decrypt_file,
            encrypt_folder,
            decrypt_folder,
            export_backup,
            import_backup,
            pick_file_path,
            pick_save_path,
            pick_folder_path,
            open_project_repository
        ])
        .build(tauri::generate_context!())
        .expect("error while building NEXUS-Q Vault")
        .run(|app, event| match event {
            RunEvent::ExitRequested { api, .. } => {
                let state = app.state::<AppState>();
                if let Err(error) = lock_session(&state) {
                    let _ = app.emit("vault-lock-failed", error);
                    api.prevent_exit();
                }
            }
            RunEvent::Resumed => {
                let state = app.state::<AppState>();
                match lock_session(&state) {
                    Ok(true) => {
                        let _ = app.emit("vault-auto-locked", "system-resume");
                    }
                    Ok(false) => {}
                    Err(error) => {
                        let _ = app.emit("vault-lock-failed", error);
                    }
                }
                let _ = touch_activity(&state);
            }
            RunEvent::Exit => app
                .state::<AppState>()
                .watchdog_stop
                .store(true, Ordering::Relaxed),
            _ => {}
        });
}

#[cfg(test)]
mod idle_lock_tests {
    use super::{
        AppState, AutoLockOutcome, auto_lock_state, idle_timeout_from_seconds, inactivity_elapsed,
        lock_session_slot, touch_activity,
    };
    use nexusq_core::vault::Vault;
    use std::{
        sync::{Arc, mpsc},
        thread,
        time::{Duration, Instant},
    };

    fn state_with_idle_unlocked_vault(dir: &tempfile::TempDir) -> AppState {
        let vault_path = dir.path().join("vault.nqv");
        Vault::create(&vault_path, b"test-password", None).unwrap();
        let session = Vault::open(&vault_path)
            .unwrap()
            .unlock(b"test-password")
            .unwrap();

        let state = AppState::default();
        *state.session.lock().unwrap() = Some(session);
        *state.idle_timeout.lock().unwrap() = Duration::from_secs(60);
        *state.last_activity.lock().unwrap() = Instant::now() - Duration::from_secs(61);
        state
    }

    #[test]
    fn accepts_only_supported_idle_timeouts() {
        for seconds in [0, 60, 300, 600, 900, 1800] {
            assert_eq!(
                idle_timeout_from_seconds(seconds),
                Ok(Duration::from_secs(seconds))
            );
        }
        for seconds in [30, 120, 3600] {
            assert!(idle_timeout_from_seconds(seconds).is_err());
        }
    }

    #[test]
    fn recent_activity_resets_the_inactivity_deadline() {
        let state = AppState::default();
        let timeout = Duration::from_secs(300);
        *state.last_activity.lock().unwrap() = Instant::now() - timeout;
        assert!(inactivity_elapsed(
            *state.last_activity.lock().unwrap(),
            Instant::now(),
            timeout
        ));

        touch_activity(&state).unwrap();
        assert!(!inactivity_elapsed(
            *state.last_activity.lock().unwrap(),
            Instant::now(),
            timeout
        ));
    }

    #[test]
    fn idle_lock_happens_only_once_for_a_session() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with_idle_unlocked_vault(&dir);

        assert_eq!(auto_lock_state(&state), AutoLockOutcome::Locked);
        assert!(state.session.lock().unwrap().is_none());
        assert_eq!(auto_lock_state(&state), AutoLockOutcome::NotDue);
    }

    #[test]
    fn idle_lock_waits_for_an_operation_holding_the_session_mutex() {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(state_with_idle_unlocked_vault(&dir));
        let operation_guard = state.session.lock().unwrap();
        let worker_state = Arc::clone(&state);
        let (started_tx, started_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();

        let worker = thread::spawn(move || {
            started_tx.send(()).unwrap();
            result_tx.send(auto_lock_state(&worker_state)).unwrap();
        });

        started_rx.recv().unwrap();
        assert!(result_rx.recv_timeout(Duration::from_millis(100)).is_err());
        drop(operation_guard);

        assert_eq!(
            result_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
            AutoLockOutcome::Locked
        );
        worker.join().unwrap();
        assert!(state.session.lock().unwrap().is_none());
    }

    #[test]
    fn failed_auto_lock_preserves_session_and_defers_retries() {
        let dir = tempfile::tempdir().unwrap();
        let state = state_with_idle_unlocked_vault(&dir);
        let vault_path = dir.path().join("vault.nqv");
        std::fs::remove_file(&vault_path).unwrap();
        std::fs::create_dir(&vault_path).unwrap();

        assert!(matches!(
            auto_lock_state(&state),
            AutoLockOutcome::Failed(_)
        ));
        assert!(state.session.lock().unwrap().is_some());
        assert_eq!(auto_lock_state(&state), AutoLockOutcome::NotDue);
    }

    #[test]
    fn successful_lock_removes_the_session_from_application_state() {
        let dir = tempfile::tempdir().unwrap();
        let vault_path = dir.path().join("vault.nqv");
        Vault::create(&vault_path, b"test-password", None).unwrap();
        let session = Vault::open(&vault_path)
            .unwrap()
            .unlock(b"test-password")
            .unwrap();

        let mut slot = Some(session);
        assert!(lock_session_slot(&mut slot).unwrap());
        assert!(slot.is_none());
    }

    #[test]
    fn failed_lock_keeps_the_session_in_application_state() {
        let dir = tempfile::tempdir().unwrap();
        let vault_path = dir.path().join("vault.nqv");
        Vault::create(&vault_path, b"test-password", None).unwrap();
        let session = Vault::open(&vault_path)
            .unwrap()
            .unlock(b"test-password")
            .unwrap();

        // Replace the vault file with a directory so the lock write fails.
        std::fs::remove_file(&vault_path).unwrap();
        std::fs::create_dir(&vault_path).unwrap();

        let mut slot = Some(session);
        assert!(lock_session_slot(&mut slot).is_err());
        assert!(
            slot.is_some(),
            "failed locking must not discard the session"
        );
    }

    #[test]
    fn inactivity_deadline_is_inclusive_and_disabled_for_zero() {
        let now = Instant::now();
        let last_activity = now - Duration::from_secs(300);
        assert!(!inactivity_elapsed(
            last_activity,
            now,
            Duration::from_secs(301)
        ));
        assert!(inactivity_elapsed(
            last_activity,
            now,
            Duration::from_secs(300)
        ));
        assert!(!inactivity_elapsed(last_activity, now, Duration::ZERO));
    }
}

#[cfg(test)]
mod password_confirmation_tests {
    use super::confirmed_secret;

    #[test]
    fn accepts_matching_passwords_without_normalizing_whitespace() {
        let password = confirmed_secret(
            "  exact password  ".into(),
            "  exact password  ".into(),
            "password",
        )
        .expect("matching values should be accepted");
        assert_eq!(password.as_slice(), b"  exact password  ");
    }

    #[test]
    fn rejects_mismatched_passwords() {
        assert!(
            confirmed_secret(
                "first password".into(),
                "second password".into(),
                "password"
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_empty_passwords_and_whitespace_mismatches() {
        assert!(confirmed_secret(String::new(), String::new(), "password").is_err());
        assert!(confirmed_secret(String::new(), "non-empty".into(), "password").is_err());
        assert!(confirmed_secret("password".into(), " password".into(), "password").is_err());
    }
}
