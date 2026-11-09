const invoke = window.__TAURI__.core.invoke;
const $ = (id) => document.getElementById(id);
const feedback = $("feedback");
const statusLabel = $("vault-status");
const statusDot = $("status-dot");
const lockButton = $("lock-button");
let vaultUnlocked = false;
const operationList = $("operation-list");
const operationCards = new Map();
const trackedCommands = new Set(["encrypt_file", "decrypt_file", "encrypt_folder", "decrypt_folder", "export_backup", "import_backup"]);
const operationTitles = {
  encrypt_file: "Encrypting file", decrypt_file: "Restoring file",
  encrypt_folder: "Encrypting folder", decrypt_folder: "Restoring folder",
  export_backup: "Creating key backup", import_backup: "Restoring key backup",
};
const operationContexts = {
  encrypt_file: "Source file → encrypted .nqx output",
  decrypt_file: "Encrypted .nqx input → restored file",
  encrypt_folder: "Source folder → encrypted folder vault",
  decrypt_folder: "Folder vault → restored folder",
  export_backup: "Vault state and keys → .nqb backup",
  import_backup: ".nqb backup → new vault",
};
const phaseTitles = {
  preparing: "Preparing", readingInput: "Reading input", processing: "Processing",
  writingOutput: "Writing output", verifying: "Verifying", completed: "Completed",
};
function createOperationCard(operationId, command) {
  if (!operationList) return;
  const empty = operationList.querySelector(".operation-empty");
  if (empty) empty.remove();
  const item = document.createElement("article");
  item.className = "operation-item";
  const title = document.createElement("span");
  title.className = "operation-item-title";
  title.textContent = operationTitles[command] || "Vault operation";
  const context = document.createElement("span");
  context.className = "operation-item-context";
  context.textContent = operationContexts[command] || "Local vault operation";
  const status = document.createElement("span");
  status.className = "operation-item-status";
  status.setAttribute("aria-live", "polite");
  status.dataset.state = "queued";
  status.textContent = preference("language", "en") === "es" ? "En cola" : "Queued";
  const phase = document.createElement("span");
  phase.className = "operation-item-phase";
  phase.textContent = "";
  const progress = document.createElement("progress");
  progress.removeAttribute("value");
  progress.max = 100;
  progress.setAttribute("aria-label", "Operation progress");
  const cancel = document.createElement("button");
  cancel.type = "button";
  cancel.textContent = preference("language", "en") === "es" ? "Cancelar" : "Cancel";
  cancel.addEventListener("click", async () => {
    cancel.disabled = true;
    cancel.textContent = preference("language", "en") === "es" ? "Solicitando…" : "Requesting…";
    try {
      const accepted = await invoke("cancel_operation", { operationId });
      if (!accepted) cancel.hidden = true;
    } catch (error) {
      cancel.disabled = false;
      cancel.textContent = preference("language", "en") === "es" ? "Cancelar" : "Cancel";
      showMessage(String(error), true);
    }
  });
  item.append(title, status, context, phase, progress, cancel);
  operationList.prepend(item);
  operationCards.set(operationId, { item, status, phase, progress, cancel });
  refreshLockButtonAvailability();
  while (operationList.children.length > 8) {
    const stale = [...operationCards.entries()].find(([, card]) =>
      ["completed", "failed", "cancelled"].includes(card.status.dataset.state));
    if (!stale) break;
    stale[1].item.remove();
    operationCards.delete(stale[0]);
  }
}
function updateOperationCard(payload) {
  const card = operationCards.get(payload.operationId);
  if (!card) return;
  const language = preference("language", "en");
  const states = language === "es"
    ? { queued: "En cola", running: "En curso", cancelling: "Cancelando", completed: "Completada", failed: "Error", cancelled: "Cancelada" }
    : { queued: "Queued", running: "Running", cancelling: "Cancelling", completed: "Completed", failed: "Failed", cancelled: "Cancelled" };
  const currentStatus = card.status.dataset.state;
  const preserveCancellation = currentStatus === "cancelling" && payload.status === "running";
  if (!preserveCancellation) {
    card.status.textContent = states[payload.status] || payload.status;
    card.status.dataset.state = payload.status;
  }
  refreshLockButtonAvailability();
  if (payload.phase) {
    const phase = phaseTitles[payload.phase] || payload.phase;
    card.phase.textContent = language === "es" ? ({ Preparing: "Preparando", "Reading input": "Leyendo entrada", Processing: "Procesando", "Writing output": "Escribiendo resultado", Verifying: "Verificando", Completed: "Completado" }[phase] || phase) : phase;
  }
  if (Number.isFinite(payload.completed) && Number.isFinite(payload.total) && payload.total > 0) {
    card.progress.max = payload.total;
    card.progress.value = Math.min(payload.completed, payload.total);
  } else if (payload.status === "running" && payload.phase !== "completed") {
    card.progress.removeAttribute("value");
  }
  if (["cancelling", "completed", "failed", "cancelled"].includes(payload.status)) {
    card.cancel.hidden = true;
    card.cancel.disabled = true;
  }
  if (["completed", "failed", "cancelled"].includes(payload.status)) {
    if (!card.progress.hasAttribute("value")) card.progress.hidden = true;
    if (payload.message) {
      const message = language === "es" ? (translations[payload.message] || payload.message) : payload.message;
      card.phase.textContent = message;
      if (payload.status === "failed") showMessage(message, true);
    }
  }
}
window.__TAURI__.event.listen("operation-update", (event) => updateOperationCard(event.payload))
  .catch((error) => showMessage(String(error), true));

function showMessage(message, isError = false) {
  feedback.hidden = false;
  feedback.classList.toggle("error", isError);
  feedback.setAttribute("role", isError ? "alert" : "status");
  feedback.setAttribute("aria-live", isError ? "assertive" : "polite");
  feedback.textContent = message;
}

function clearSecrets(form) {
  form.querySelectorAll(".secret-row").forEach((row) => {
    const input = row.querySelector("input");
    const toggle = row.querySelector("button");
    input.value = "";
    input.type = "password";
    input.setCustomValidity("");
    toggle.textContent = preference("language", "en") === "es" ? "Mostrar" : "Show";
    toggle.setAttribute("aria-pressed", "false");
    toggle.setAttribute("aria-label", preference("language", "en") === "es" ? "Mostrar contraseña" : "Show password");
  });
}

function clearAllSecrets() {
  document.querySelectorAll(".secret-row").forEach((row) => {
    const input = row.querySelector("input");
    const toggle = row.querySelector("button");
    if (!input || !toggle) return;
    input.value = "";
    input.type = "password";
    input.setCustomValidity("");
    toggle.textContent = preference("language", "en") === "es" ? "Mostrar" : "Show";
    toggle.setAttribute("aria-pressed", "false");
    toggle.setAttribute("aria-label", preference("language", "en") === "es" ? "Mostrar contraseña" : "Show password");
  });
}

function bindSecretToggle(buttonId, inputId) {
  const button = $(buttonId);
  const input = $(inputId);
  button.addEventListener("click", () => {
    const reveal = input.type === "password";
    input.type = reveal ? "text" : "password";
    button.textContent = reveal ? (preference("language", "en") === "es" ? "Ocultar" : "Hide") : (preference("language", "en") === "es" ? "Mostrar" : "Show");
    button.setAttribute("aria-pressed", String(reveal));
    button.setAttribute("aria-label", `${reveal ? (preference("language", "en") === "es" ? "Ocultar" : "Hide") : (preference("language", "en") === "es" ? "Mostrar" : "Show")} ${preference("language", "en") === "es" ? "contraseña" : "password"}`);
  });
}

function refreshLockButtonAvailability() {
  const hasActiveOperations = [...operationCards.values()].some(({ status }) =>
    status.dataset.state === "queued" || status.dataset.state === "running" || status.dataset.state === "cancelling");
  lockButton.disabled = !vaultUnlocked || hasActiveOperations;
}

async function refreshStatus() {
  try {
    const unlocked = await invoke("vault_status");
    vaultUnlocked = unlocked;
    statusLabel.textContent = preference("language", "en") === "es" ? (unlocked ? "Desbloqueado" : "Bloqueado") : (unlocked ? "Unlocked" : "Locked");
    statusDot.classList.toggle("active", unlocked);
    refreshLockButtonAvailability();
    document.querySelectorAll(".workspace input,.workspace button[type=submit],.workspace .picker,.workspace .reveal")
      .forEach((el) => { el.disabled = !unlocked; });
  } catch (error) {
    showMessage(String(error), true);
  }
}

function submit(formId, command, fields) {
  const form = $(formId);
  let submittedOperationId = null;
  form.querySelectorAll("[data-confirm-for]").forEach((confirmation) => {
    const password = $(confirmation.dataset.confirmFor);
    [password, confirmation].forEach((input) => {
      input.addEventListener("input", () => confirmation.setCustomValidity(""));
    });
  });
  const handleConfirmedSubmit = passwordConfirmation.createConfirmedFormSubmit({
    readConfirmations: () => [...form.querySelectorAll("[data-confirm-for]")].map((confirmation) => ({
      input: confirmation,
      value: $(confirmation.dataset.confirmFor).value,
      confirmation: confirmation.value,
    })),
    onMismatch: ({ input }) => {
      const message = preference("language", "en") === "es"
        ? "Las contraseñas no coinciden. Comprueba ambos campos."
        : "Passwords do not match. Check both fields.";
      input.setCustomValidity(message);
      input.reportValidity();
      input.focus();
      showMessage(message, true);
    },
    onMatch: () => {
      const payload = Object.fromEntries(fields.map(([key, id]) => [key, $(id).value]));
      if (trackedCommands.has(command)) {
        const operationId = crypto.randomUUID();
        submittedOperationId = operationId;
        createOperationCard(operationId, command);
        return invoke(command, { ...payload, operationId });
      }
      return invoke(command, payload);
    },
  });
  form.addEventListener("submit", async (event) => {
    const validation = handleConfirmedSubmit(event);
    if (!validation.matched) return;
    const confirmations = [...form.querySelectorAll("[data-confirm-for]")];
    confirmations.forEach((confirmation) => confirmation.setCustomValidity(""));
    const button = form.querySelector('button[type="submit"]');
    const originalLabel = button.textContent;
    button.disabled = true;
    button.textContent = preference("language", "en") === "es" ? "Procesando…" : "Processing…";
    form.setAttribute("aria-busy", "true");
    showMessage("Operación en curso. Espera a que termine.");
    try {
      showMessage(await validation.result);
    } catch (error) {
      if (submittedOperationId) {
        const card = operationCards.get(submittedOperationId);
        if (card && ["queued", "running"].includes(card.status.dataset.state)) {
          updateOperationCard({ operationId: submittedOperationId, status: "failed" });
        }
      }
      showMessage(String(error), true);
    } finally {
      clearSecrets(form);
      form.removeAttribute("aria-busy");
      button.textContent = originalLabel;
      button.disabled = false;
      await refreshStatus();
    }
  });
}

function childPath(parent, childName) {
  const isWindowsPath = /^[A-Za-z]:[\\/]/.test(parent) || parent.startsWith("\\\\");
  const separator = isWindowsPath ? "\\" : "/";
  return `${parent.replace(/[\\/]+$/, "")}${separator}${childName}`;
}

function bindPicker(buttonId, inputId, command, kind, transform = (path) => path) {
  const button = $(buttonId);
  button.addEventListener("click", async () => {
    const originalLabel = button.textContent;
    button.disabled = true;
    button.textContent = preference("language", "en") === "es" ? "Abriendo…" : "Opening…";
    try {
      const selectedPath = await invoke(command, { kind });
      if (selectedPath) {
        $(inputId).value = transform(selectedPath);
        showMessage("Ruta seleccionada.");
      }
    } catch (error) {
      showMessage(String(error), true);
    } finally {
      button.textContent = originalLabel;
      button.disabled = false;
      await refreshStatus();
    }
  });
}

submit("create-form", "create_vault", [["path", "create-path"], ["password", "create-password"], ["passwordConfirmation", "create-password-confirm"]]);
submit("open-form", "open_vault", [["path", "open-path"], ["password", "open-password"]]);
submit("encrypt-form", "encrypt_file", [["inputPath", "encrypt-path"]]);
submit("decrypt-form", "decrypt_file", [["inputPath", "decrypt-input"], ["outputPath", "decrypt-output"]]);
submit("encrypt-folder-form", "encrypt_folder", [["sourcePath", "encrypt-folder-source"], ["vaultFolderPath", "encrypt-folder-output"]]);
submit("decrypt-folder-form", "decrypt_folder", [["vaultFolderPath", "decrypt-folder-source"], ["destinationPath", "decrypt-folder-output"]]);
submit("backup-form", "export_backup", [["backupPath", "backup-path"], ["backupPassphrase", "backup-password"], ["backupPassphraseConfirmation", "backup-password-confirm"]]);
submit("restore-form", "import_backup", [["backupPath", "restore-backup-path"], ["backupPassphrase", "restore-backup-password"], ["newVaultPath", "restore-vault-path"], ["newVaultPassword", "restore-vault-password"], ["newVaultPasswordConfirmation", "restore-vault-password-confirm"]]);

bindSecretToggle("create-password-toggle", "create-password");
bindSecretToggle("create-password-confirm-toggle", "create-password-confirm");
bindSecretToggle("open-password-toggle", "open-password");
bindSecretToggle("backup-password-toggle", "backup-password");
bindSecretToggle("backup-password-confirm-toggle", "backup-password-confirm");
bindSecretToggle("restore-backup-password-toggle", "restore-backup-password");
bindSecretToggle("restore-vault-password-toggle", "restore-vault-password");
bindSecretToggle("restore-vault-password-confirm-toggle", "restore-vault-password-confirm");

bindPicker("create-path-picker", "create-path", "pick_save_path", "vault");
bindPicker("open-path-picker", "open-path", "pick_file_path", "vault");
bindPicker("encrypt-path-picker", "encrypt-path", "pick_file_path", "any");
bindPicker("decrypt-input-picker", "decrypt-input", "pick_file_path", "encrypted");
bindPicker("decrypt-output-picker", "decrypt-output", "pick_save_path", "restored");
bindPicker("encrypt-folder-source-picker", "encrypt-folder-source", "pick_folder_path", "source");
bindPicker("encrypt-folder-output-picker", "encrypt-folder-output", "pick_folder_path", "parent",
  (path) => childPath(path, "NEXUS-Q-FolderVault"));
bindPicker("decrypt-folder-source-picker", "decrypt-folder-source", "pick_folder_path", "vault-folder");
bindPicker("decrypt-folder-output-picker", "decrypt-folder-output", "pick_folder_path", "parent",
  (path) => childPath(path, "Restored-Folder"));
bindPicker("backup-path-picker", "backup-path", "pick_save_path", "backup");
bindPicker("restore-backup-path-picker", "restore-backup-path", "pick_file_path", "backup");
bindPicker("restore-vault-path-picker", "restore-vault-path", "pick_save_path", "vault");

lockButton.addEventListener("click", async () => {
  const originalLabel = lockButton.textContent;
  lockButton.disabled = true;
  lockButton.textContent = preference("language", "en") === "es" ? "Bloqueando…" : "Locking…";
  try {
    showMessage(await invoke("lock_vault"));
  } catch (error) {
    showMessage(String(error), true);
  } finally {
    clearAllSecrets();
    lockButton.textContent = originalLabel;
    await refreshStatus();
  }
});

refreshStatus();

// Local preferences: language and profile never leave this device.
const preference = (key, fallback = "") => { try { return localStorage.getItem(`nqv.${key}`) ?? fallback; } catch { return fallback; } };
const storePreference = (key, value) => { try { localStorage.setItem(`nqv.${key}`, value); } catch { /* storage may be unavailable */ } };
let lastActivitySent = 0;
function reportActivity() {
  const now = Date.now();
  if (now - lastActivitySent < 10000) return;
  lastActivitySent = now;
  invoke("record_activity").catch(() => {});
}
["pointerdown", "pointermove", "keydown", "touchstart", "wheel"].forEach((eventName) => {
  document.addEventListener(eventName, reportActivity, { passive: true });
});
document.addEventListener("visibilitychange", async () => {
  if (document.hidden) {
    try {
      await invoke("lock_vault");
    } catch (error) {
      if (!String(error).includes("No hay ningún vault desbloqueado")) showMessage(String(error), true);
    } finally {
      clearAllSecrets();
      await refreshStatus();
    }
  } else {
    await refreshStatus();
  }
});
const idleTimeoutSelect = $("idle-timeout");
async function initializeIdleTimeout() {
  if (!idleTimeoutSelect) return;
  try {
    const stored = Number(preference("idleTimeout", "300"));
    const allowed = [0, 60, 300, 600, 900, 1800];
    const seconds = allowed.includes(stored) ? stored : 300;
    await invoke("set_idle_timeout", { seconds });
    idleTimeoutSelect.value = String(seconds);
  } catch (error) { showMessage(String(error), true); }
}
idleTimeoutSelect?.addEventListener("change", async () => {
  const seconds = Number(idleTimeoutSelect.value);
  idleTimeoutSelect.disabled = true;
  try {
    await invoke("set_idle_timeout", { seconds });
    storePreference("idleTimeout", String(seconds));
    showMessage(preference("language", "en") === "es" ? "Bloqueo automático actualizado." : "Automatic lock updated.");
  } catch (error) { showMessage(String(error), true); }
  finally { idleTimeoutSelect.disabled = false; }
});
window.__TAURI__.event.listen("vault-auto-locked", async () => {
  clearAllSecrets();
  await refreshStatus();
  showMessage(preference("language", "en") === "es" ? "El vault se bloqueó automáticamente por seguridad." : "The vault was automatically locked for security.");
}).catch((error) => showMessage(String(error), true));
window.__TAURI__.event.listen("vault-lock-failed", (event) => {
  clearAllSecrets();
  showMessage(String(event.payload), true);
  refreshStatus();
}).catch((error) => showMessage(String(error), true));
initializeIdleTimeout();

const translations = {
  "LOCAL-FIRST SECURITY":"SEGURIDAD LOCAL", "YOUR PRIVATE SPACE":"TU ESPACIO PRIVADO", "Protect your files.":"Protege tus archivos.", "Without leaving your device.":"Sin salir de tu dispositivo.",
  "Local encryption and protected backups. Your original files are kept; Vault never deletes them automatically.":"Cifrado local y backups protegidos. Tus archivos originales se conservan; Vault nunca los elimina automáticamente.",
  "Create a vault":"Crear un vault", "Generate a new encrypted container.":"Genera un contenedor cifrado nuevo.", "Vault location":"Ruta del vault", "Vault password":"Contraseña del vault", "Create & unlock":"Crear y desbloquear",
  "Open a vault":"Abrir un vault", "Unlock an existing container.":"Desbloquea un contenedor existente.", "Vault path":"Ruta del vault", "Password":"Contraseña", "Unlock":"Desbloquear",
  "UNLOCKED VAULT":"VAULT DESBLOQUEADO", "Secure operations":"Operaciones seguras", "Lock vault":"Bloquear vault", "Encrypt file":"Cifrar archivo", "The encrypted file is created next to the original with the .nqx extension.":"El archivo cifrado se crea junto al original con extensión .nqx.", "Original file":"Archivo original", "Restore file":"Restaurar archivo", "The destination must be new; files are never silently overwritten.":"El destino debe ser nuevo; nunca se sobrescribe silenciosamente.", "Encrypted .nqx file":"Archivo .nqx", "Restored destination":"Destino restaurado", "Encrypt folder":"Cifrar carpeta", "Names and paths are stored in an encrypted manifest. Originals are not modified.":"Los nombres y las rutas se guardan en un manifiesto cifrado. No modifica los originales.", "Original folder":"Carpeta original", "New vault folder":"Nueva carpeta vault", "A new name is suggested inside the selected folder; you can edit the path.":"Se propone un nombre dentro de la carpeta elegida; puedes editar la ruta.", "Encrypt entire folder":"Cifrar carpeta completa", "Restore folder":"Restaurar carpeta", "The destination folder must be new. The manifest and every encrypted object are validated.":"La carpeta de destino debe ser nueva. Se validan el manifiesto y cada objeto cifrado.", "Vault folder":"Carpeta vault", "Restored destination":"Destino restaurado", "A new folder is suggested inside the selected location.":"Se propone una carpeta nueva dentro de la ubicación elegida.", "Restore entire folder":"Restaurar carpeta completa", "Create backup":"Crear backup", "Protects the container and its keys; it does not include .nqx files or protected folders.":"Protege el contenedor y sus claves; no incluye los archivos .nqx ni las carpetas protegidas.", "Backup location":"Ruta del backup", "Backup password":"Contraseña del backup", "Create key backup":"Crear backup de claves", "Restore backup":"Restaurar backup", "Restores keys into a new vault. Encrypted files are stored separately.":"Restaura las claves en un vault nuevo. Los archivos cifrados se conservan aparte.", "Backup .nqb":"Backup .nqb", "New vault location":"Ruta del nuevo vault", "New vault password":"Nueva contraseña del vault", "Restore to new vault":"Restaurar en nuevo vault", "Browse":"Examinar", "Show":"Mostrar", "Hide":"Ocultar", "Locked":"Bloqueado", "Unlocked":"Desbloqueado", "Processing…":"Procesando…", "Operation in progress. Please wait.":"Operación en curso. Espera a que termine.", "Path selected.":"Ruta seleccionada.", "Block vault":"Bloquear vault", "Password":"Contraseña", "Show password":"Mostrar contraseña", "Hide password":"Ocultar contraseña"
};
Object.assign(translations, {
  "YOUR PRIVATE SPACE":"TU ESPACIO PRIVADO", "Protect your files.":"Protege tus archivos.", "Without leaving your device.":"Sin salir de tu dispositivo.",
  "Local encryption and protected backups. Your original files are kept: Vault never deletes them automatically.":"Cifrado local y backups protegidos. Tus archivos originales se conservan: Vault nunca los elimina automáticamente.",
  "Create a vault":"Crear un vault", "Generate a new encrypted container.":"Genera un contenedor cifrado nuevo.", "Vault location":"Ruta del nuevo vault", "Confirm password":"Confirmar contraseña", "Vault password":"Contraseña del vault", "Password":"Contraseña", "Create & unlock":"Crear y desbloquear",
  "Open a vault":"Abrir un vault", "Unlock an existing container.":"Desbloquea un contenedor existente.", "Unlock":"Desbloquear", "UNLOCKED VAULT":"VAULT DESBLOQUEADO", "Secure operations":"Operaciones seguras", "Lock vault":"Bloquear vault",
  "Encrypt file":"Cifrar archivo", "The encrypted file is created next to the original with the .nqx extension.":"El archivo cifrado se crea junto al original con extensión .nqx.", "Original file":"Archivo original", "Restore file":"Restaurar archivo", "The destination must be new; files are never silently overwritten.":"El destino debe ser nuevo; nunca se sobrescribe silenciosamente.", "Encrypted .nqx file":"Archivo .nqx", "Restored destination":"Destino restaurado",
  "Encrypt folder":"Cifrar carpeta", "Names and paths are stored in an encrypted manifest. Originals are not modified.":"Los nombres y las rutas se guardan en un manifiesto cifrado. No modifica los originales.", "Original folder":"Carpeta original", "New vault folder":"Nueva carpeta vault", "A new name is suggested inside the selected folder; you can edit the path.":"Se propone un nombre dentro de la carpeta elegida; puedes editar la ruta.", "Encrypt entire folder":"Cifrar carpeta completa",
  "Restore folder":"Restaurar carpeta", "The destination folder must be new. The manifest and every encrypted object are validated.":"La carpeta de destino debe ser nueva. Se validan el manifiesto y cada objeto cifrado.", "Vault folder":"Carpeta vault", "A new folder is suggested inside the selected location.":"Se propone una carpeta nueva dentro de la ubicación elegida.", "Restore entire folder":"Restaurar carpeta completa",
  "Create backup":"Crear backup", "Protects the container and its keys; it does not include .nqx files or protected folders.":"Protege el contenedor y sus claves; no incluye los archivos .nqx ni las carpetas protegidas.", "Backup location":"Ruta del backup", "Backup password":"Contraseña del backup", "Confirm backup password":"Confirmar contraseña del backup", "Create key backup":"Crear backup de claves",
  "Restore backup":"Restaurar backup", "Restores keys into a new vault. Encrypted files are stored separately.":"Restaura las claves en un vault nuevo. Los archivos cifrados se conservan aparte.", "Backup .nqb":"Backup .nqb", "New vault password":"Nueva contraseña del vault", "Confirm new password":"Confirmar nueva contraseña", "Restore to new vault":"Restaurar en nuevo vault",
  "Create vault-key backup":"Crear backup de claves del vault", "The .nqb file protects vault state and keys; it is not a copy of your documents.":"El archivo .nqb protege el estado del vault y sus claves; no es una copia de tus documentos.", "What this backup contains":"Qué guarda este backup", "Includes:":"Incluye:", "Encrypted vault state and the key material needed to recover it.":"El estado cifrado del vault y el material de claves necesario para recuperarlo.", "Does not include:":"No incluye:", "Individual .nqx files, manifest.nqm files or encrypted folder objects.":"Archivos .nqx, manifiestos manifest.nqm ni objetos de carpetas cifradas.", "Keep the .nqb and copies of every .nqx file and encrypted folder separately. The backup passphrase is independent of the vault password; store it away from the backup file.":"Conserva por separado el .nqb y copias de todos tus archivos .nqx y carpetas cifradas. La contraseña del backup es independiente de la contraseña del vault; guárdala aparte del archivo.", "Restore vault-key backup":"Restaurar backup de claves", "Import vault state from a .nqb file and create a new vault.":"Importa el estado del vault desde un .nqb y crea un vault nuevo.", "Before restoring: you need these two recovery components":"Antes de restaurar: necesitas estos dos componentes de recuperación", "Enter the separate passphrase used to create the .nqb file.":"Introduce la contraseña independiente con la que se creó el .nqb.", "The import creates a new vault protected by the new password you choose here.":"La importación crea un vault nuevo protegido con la contraseña nueva que elijas aquí.", "Required: the .nqb vault-key backup and its separate passphrase.":"Necesitas el backup de claves del vault (.nqb) y su contraseña independiente.", "Also required: separately preserved encrypted payloads (.nqx files and encrypted folder data).":"También necesitas los documentos cifrados conservados por separado (archivos .nqx y datos de carpetas cifradas).", "The restored keys are used to decrypt the separately preserved .nqx files and restore the encrypted folder data.":"Después de importar el .nqb en un vault nuevo, utiliza las claves restauradas para descifrar los archivos .nqx y restaurar los datos de las carpetas cifradas.", "The .nqb does not contain encrypted documents and cannot reveal, recover or reset the original vault password. Without the .nqb, its passphrase and the separately preserved encrypted payloads, recovery may be impossible.":"El .nqb no contiene documentos cifrados y no revela, recupera ni restablece la contraseña original del vault. Sin el .nqb, su contraseña y los payloads cifrados conservados por separado, la recuperación puede ser imposible.", "Make a vault-key backup":"Crea un backup de claves del vault", "The encrypted .nqb stores vault/key state only. It excludes .nqx files, folder manifests and encrypted folder objects. Keep all encrypted documents separately and store the backup passphrase away from the .nqb file.":"El .nqb cifrado solo guarda el estado y las claves del vault. Excluye los archivos .nqx, los manifiestos de carpetas y sus objetos cifrados. Conserva todos los documentos cifrados por separado y guarda la contraseña del backup aparte del archivo .nqb.", "Restore keys and documents":"Restaura las claves y los documentos", "Import the .nqb with its separate backup passphrase and create a new vault with a new vault password. The restored keys are then used to decrypt your separately preserved .nqx files and restore your encrypted folder data. This does not reveal or reset the original vault password.":"Importa el .nqb con su contraseña independiente y crea un vault nuevo con una contraseña nueva. Después utiliza las claves restauradas para descifrar los archivos .nqx y restaurar los datos de las carpetas cifradas que conservaste por separado. Esto no revela ni restablece la contraseña original del vault.",
  "THE PROBLEM":"EL PROBLEMA", "Your information deserves more than trusting a folder.":"Tu información merece algo más que confiar en una carpeta.", "A device can be lost, shared or exposed. Sensitive documents often end up mixed with everyday files, without a clear layer of protection or an orderly way to recover access.":"Un dispositivo puede perderse, compartirse o quedar expuesto. Los documentos sensibles suelen acabar mezclados con archivos cotidianos, sin una capa clara de protección ni una forma ordenada de recuperar el acceso.", "Local protection":"Protección local", "Encrypt files and folders from your computer. Cryptographic work happens in the app; your originals remain untouched.":"Cifra archivos y carpetas desde tu equipo. El trabajo criptográfico se realiza en la aplicación; tus originales permanecen intactos.", "Recovery, planned":"Recuperación preparada", "Create password-protected .nqb backups with a separate passphrase to restore keys into a new vault.":"Crea backups .nqb protegidos con una contraseña independiente para restaurar las claves en un vault nuevo.", "Transparent control":"Control transparente", "Choose where each vault, encrypted file and backup lives. No telemetry and no automatic deletion of originals.":"Elige dónde guardar cada vault, cada archivo cifrado y cada backup. Sin telemetría y sin borrar automáticamente los originales.", "A SIMPLE WORKFLOW":"UN FLUJO SENCILLO", "From everyday file to protected file.":"De archivo normal a archivo protegido.", "Create or open your vault":"Crea o abre tu vault", "Unlock it":"Desbloquéalo", "Encrypt your files":"Cifra tus archivos", "Save a backup":"Guarda un backup", "One important thing":"Una idea importante", "The vault protects the keys; .nqx files and encrypted folders are stored separately. Keep your password and backup in safe places too.":"El vault protege las claves; los archivos .nqx y las carpetas cifradas se guardan por separado. Conserva también tu contraseña y tu backup en lugares seguros.", "Read the user guide →":"Leer la guía de usuario →", "WORKSPACE":"ESPACIO DE TRABAJO", "Overview":"Resumen", "Vault access":"Acceso al vault", "File operations":"Operaciones con archivos", "Settings":"Ajustes", "Help & guide":"Ayuda y guía", "OPEN SOURCE":"CÓDIGO ABIERTO", "Build security with us.":"Construyamos seguridad juntos.", "Explore the project, contribute ideas and leave a star on GitHub.":"Explora el proyecto, colabora y deja una estrella en GitHub.", "View NEXUS-Q repository ↗":"Ver repositorio NEXUS-Q ↗", "Local user":"Usuario local", "LOCAL PROFILE":"PERFIL LOCAL", "LOCAL-FIRST SECURITY":"SEGURIDAD LOCAL", "Light mode":"Modo claro", "Dark mode":"Modo oscuro",
  "PREFERENCES":"PREFERENCIAS", "Settings & profile":"Ajustes y perfil", "Public release · v0.1.0":"Lanzamiento público · v0.1.0", "Local profile":"Perfil local", "Your profile stays on this device and is not uploaded.":"Tu perfil permanece en este dispositivo y no se sube a ningún servidor.", "Display name":"Nombre visible", "Profile photo":"Foto de perfil", "Stored locally in this app.":"Guardada localmente en esta aplicación.", "Save profile":"Guardar perfil", "Appearance":"Apariencia", "Make Vault feel like your workspace.":"Adapta Vault a tu espacio de trabajo.", "Accent color":"Color de acento", "Typography":"Tipografía", "System minimal":"Sistema minimalista", "Elegant serif":"Serif elegante", "Monospace":"Monoespaciada", "The default palette is monochrome. Your accent and font choices are saved locally.":"La paleta predeterminada es monocromática. Tus colores y tipografía se guardan localmente.", "Apply appearance":"Aplicar apariencia", "Application":"Aplicación", "Build information and important limitations.":"Información de versión y limitaciones importantes.", "Version":"Versión", "0.1.0 · Public release":"0.1.0 · Lanzamiento público", "Data location":"Ubicación de los datos", "On this device":"En este dispositivo", "Telemetry":"Telemetría", "Disabled":"Desactivada", "This early-access release has not been independently audited. Keep separate backups of encrypted payloads and recovery material.":"Esta versión de acceso anticipado no ha sido auditada de forma independiente. Conserva copias separadas de los archivos cifrados y del material de recuperación.",
  "GETTING STARTED":"PRIMEROS PASOS", "Help & user guide":"Ayuda y guía de usuario", "Create your vault":"Crea tu vault", "Choose a location, create a strong unique password and keep it safe. The vault file stores your protected key material.":"Elige una ubicación, crea una contraseña única y robusta y guárdala de forma segura. El archivo vault almacena tus claves protegidas.", "Unlock your vault":"Desbloquea tu vault", "Select your existing .nqv file and enter its password. Lock the vault whenever you leave your computer.":"Selecciona tu archivo .nqv e introduce su contraseña. Bloquea el vault cuando te alejes del ordenador.", "Protect files and folders":"Protege archivos y carpetas", "Unlock first, then choose a file or folder to encrypt. Original files are not deleted automatically.":"Primero desbloquea el vault y después elige un archivo o carpeta para cifrar. Los originales no se eliminan automáticamente.", "Make a recovery backup":"Crea un backup de recuperación", "Create an encrypted .nqb backup with a separate strong passphrase. Store it separately from your encrypted files.":"Crea un backup .nqb cifrado con una contraseña independiente y robusta. Guárdalo separado de tus archivos cifrados.", "Restore safely":"Restaura de forma segura", "Use the backup passphrase to restore into a new vault. Keep the encrypted .nqx files and folder-vault data separately.":"Utiliza la contraseña del backup para restaurar en un vault nuevo. Conserva por separado los archivos .nqx y los datos de las carpetas vault.", "Important limitations":"Limitaciones importantes", "Folder encryption currently supports files up to 32 MiB, skips empty folders and rejects symbolic links. This early-access release has not been independently audited.":"El cifrado de carpetas admite archivos de hasta 32 MiB, omite carpetas vacías y rechaza enlaces simbólicos. Esta versión de acceso anticipado no ha sido auditada de forma independiente.", "Made for privacy":"Diseñado para la privacidad",
  "Browse":"Examinar", "Show":"Mostrar", "Hide":"Ocultar", "Generate strong password":"Generar contraseña segura", "Store your password in a password manager. An encrypted recovery backup can restore vault keys, but it cannot reveal your original password.":"Guarda tu contraseña en un gestor de contraseñas. Un backup cifrado puede restaurar las claves del vault, pero no revelar la contraseña original."
});
Object.assign(translations, {
  "Automatic vault lock": "Bloqueo automático del vault",
  "Lock after inactivity": "Bloquear tras inactividad",
  "Lock the vault after a period without keyboard or pointer activity. This setting stays on this device.": "Bloquea el vault tras un periodo sin actividad de teclado o ratón. Este ajuste permanece en este dispositivo.",
  "Never": "Nunca",
  "1 minute": "1 minuto",
  "5 minutes": "5 minutos",
  "10 minutes": "10 minutos",
  "15 minutes": "15 minutos",
  "30 minutes": "30 minutos",
  "The countdown is enforced by the Rust backend. Closing the app also locks the vault. Desktop operating systems do not expose a portable screen-lock or suspend event through Tauri.": "La cuenta atrás la controla Rust. Cerrar la aplicación también bloquea el vault. Los sistemas de escritorio no exponen mediante Tauri un evento portable de bloqueo de pantalla o suspensión."
});
Object.assign(translations, {
  "Operations center": "Centro de operaciones",
  "Backend-reported progress; safe cancellation where supported.": "Progreso comunicado por el motor; cancelación segura cuando está disponible.",
  "No operations yet.": "Todavía no hay operaciones.",
  "Encrypting file": "Cifrando archivo", "Restoring file": "Restaurando archivo",
  "Encrypting folder": "Cifrando carpeta", "Restoring folder": "Restaurando carpeta",
  "Creating key backup": "Creando backup de claves", "Restoring key backup": "Restaurando backup de claves",
  "Source file → encrypted .nqx output": "Archivo original → archivo cifrado .nqx",
  "Encrypted .nqx input → restored file": "Archivo .nqx cifrado → archivo restaurado",
  "Source folder → encrypted folder vault": "Carpeta original → vault de carpeta cifrado",
  "Folder vault → restored folder": "Vault de carpeta → carpeta restaurada",
  "Vault state and keys → .nqb backup": "Estado y claves del vault → backup .nqb",
  ".nqb backup → new vault": "Backup .nqb → vault nuevo",
  "Queued": "En cola", "Running": "En curso", "Cancelling": "Cancelando", "Completed": "Completada", "Failed": "Error", "Cancelled": "Cancelada",
  "Preparing": "Preparando", "Reading input": "Leyendo entrada", "Processing": "Procesando",
  "Writing output": "Escribiendo resultado", "Verifying": "Verificando", "Cancel": "Cancelar",
  "Cancellation completed at a safe checkpoint; incomplete output was removed or never published.": "Cancelación completada en un punto seguro; el resultado incompleto se eliminó o no llegó a publicarse.",
  "Operation failed. Cleanup was attempted where needed; see the form feedback for details.": "La operación falló. Se intentó limpiar los resultados cuando correspondía; consulta el mensaje del formulario."
});

const themeToggle = $("theme-toggle");
function translatePage(language) {
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  let node;
  while ((node = walker.nextNode())) {
    const raw = node.nodeValue;
    const trimmed = raw.trim();
    if (!trimmed) continue;
    const replacement = language === "es" ? translations[trimmed] : Object.keys(translations).find((key) => translations[key] === trimmed);
    if (replacement) node.nodeValue = raw.replace(trimmed, replacement);
  }
  document.querySelectorAll("input[placeholder]").forEach((input) => {
    if (language === "es") {
      if (input.placeholder.includes("/path/to")) input.placeholder = input.placeholder.replace("/path/to", "/ruta/a");
      if (input.placeholder === "At least 12 bytes") input.placeholder = "Mínimo 12 bytes";
      if (input.placeholder === "Your password") input.placeholder = "Tu contraseña";
    } else {
      input.placeholder = input.placeholder.replace("/ruta/a", "/path/to");
      if (input.placeholder === "Mínimo 12 bytes") input.placeholder = "At least 12 bytes";
      if (input.placeholder === "Tu contraseña") input.placeholder = "Your password";
    }
  });
  document.documentElement.lang = language;
  const toggle = $("language-toggle");
  if (toggle) toggle.textContent = language === "en" ? "ES" : "EN";
  storePreference("language", language);
  if (themeToggle) applyTheme(preference("theme", "light"));
}
const languageToggle = $("language-toggle");
if (languageToggle) {
  languageToggle.addEventListener("click", () => translatePage(preference("language", "en") === "en" ? "es" : "en"));
  translatePage(preference("language", "en"));
}
function applyTheme(theme) {
  const dark = theme === "dark";
  document.body.classList.toggle("dark", dark);
  if (themeToggle) {
    const language = preference("language", "en");
    themeToggle.textContent = dark ? (language === "es" ? "Modo claro" : "Light mode") : (language === "es" ? "Modo oscuro" : "Dark mode");
    themeToggle.setAttribute("aria-label", dark ? (language === "es" ? "Cambiar a modo claro" : "Switch to light mode") : (language === "es" ? "Cambiar a modo oscuro" : "Switch to dark mode"));
  }
  storePreference("theme", theme);
}
applyTheme(preference("theme", "light"));
themeToggle?.addEventListener("click", () => applyTheme(document.body.classList.contains("dark") ? "light" : "dark"));
function applyProfile(name, photo) {
  const safeName = name || "Local user";
  $("sidebar-user").textContent = safeName;
  const initial = safeName.trim().charAt(0).toUpperCase() || "U";
  [$("sidebar-avatar"), $("profile-preview")].forEach((avatar) => {
    if (!avatar) return;
    avatar.textContent = "";
    if (photo) { const img = document.createElement("img"); img.src = photo; img.alt = "Profile"; avatar.append(img); }
    else avatar.textContent = initial;
  });
}
let profilePhoto = preference("profilePhoto");
$("profile-name").value = preference("profileName");
applyProfile(preference("profileName"), profilePhoto);
$("profile-photo")?.addEventListener("change", (event) => {
  const file = event.target.files?.[0];
  if (!file) return;
  if (file.size > 2 * 1024 * 1024) { showMessage("Choose an image smaller than 2 MiB.", true); event.target.value = ""; return; }
  const reader = new FileReader();
  reader.onload = () => { profilePhoto = String(reader.result); applyProfile($("profile-name").value, profilePhoto); };
  reader.readAsDataURL(file);
});
$("save-profile")?.addEventListener("click", () => {
  const name = $("profile-name").value.trim().slice(0, 40);
  storePreference("profileName", name); storePreference("profilePhoto", profilePhoto);
  applyProfile(name, profilePhoto); showMessage("Local profile saved.");
});
// Sidebar navigation switches independent views instead of scrolling one long page.
const pageLinks = [...document.querySelectorAll(".side-nav a")];
const pages = [...document.querySelectorAll(".app-page")];
function showPage(pageId, updateHash = true) {
  const selected = pages.some((page) => page.id === pageId) ? pageId : "overview";
  pages.forEach((page) => page.classList.toggle("active", page.id === selected));
  pageLinks.forEach((link) => {
    const active = link.getAttribute("href") === "#" + selected;
    link.classList.toggle("active", active);
    if (active) link.setAttribute("aria-current", "page");
    else link.removeAttribute("aria-current");
  });
  if (updateHash) history.replaceState(null, "", "#" + selected);
  window.scrollTo({ top: 0, behavior: "instant" });
}
document.querySelectorAll("[data-go-page]").forEach((link) => link.addEventListener("click", (event) => { event.preventDefault(); showPage(link.dataset.goPage); }));
pageLinks.forEach((link) => link.addEventListener("click", (event) => {
  event.preventDefault();
  showPage(link.hash.slice(1));
}));
showPage(location.hash.slice(1) || "overview", false);

document.querySelector("[data-open-repository]")?.addEventListener("click", async (event) => {
  event.preventDefault();
  try {
    await window.__TAURI__.core.invoke("open_project_repository");
  } catch (error) {
    showMessage("Could not open GitHub: " + error, true);
  }
});

// Generate a high-entropy password locally using the Web Crypto API.
$("generate-password")?.addEventListener("click", () => {
  const alphabet = "ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%^&*_-+=?";
  const limit = Math.floor(256 / alphabet.length) * alphabet.length;
  let result = "";
  while (result.length < 24) {
    const bytes = crypto.getRandomValues(new Uint8Array(32));
    for (const byte of bytes) if (byte < limit && result.length < 24) result += alphabet[byte % alphabet.length];
  }
  const input = $("create-password");
  input.value = result;
  input.type = "text";
  $("create-password-toggle").textContent = "Hide";
  $("create-password-toggle").setAttribute("aria-pressed", "true");
  showMessage(preference("language", "en") === "es" ? "Se generó una contraseña segura localmente. Guárdala en un gestor de contraseñas antes de crear el vault." : "A strong password was generated locally. Save it in a password manager before creating the vault.");
});
