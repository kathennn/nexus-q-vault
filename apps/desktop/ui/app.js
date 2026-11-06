const invoke = window.__TAURI__.core.invoke;
const $ = (id) => document.getElementById(id);
const feedback = $("feedback");
const statusLabel = $("vault-status");
const statusDot = $("status-dot");
const lockButton = $("lock-button");

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

async function refreshStatus() {
  try {
    const unlocked = await invoke("vault_status");
    statusLabel.textContent = preference("language", "en") === "es" ? (unlocked ? "Desbloqueado" : "Bloqueado") : (unlocked ? "Unlocked" : "Locked");
    statusDot.classList.toggle("active", unlocked);
    lockButton.disabled = !unlocked;
    document.querySelectorAll(".workspace input,.workspace button[type=submit],.workspace .picker,.workspace .reveal")
      .forEach((el) => { el.disabled = !unlocked; });
  } catch (error) {
    showMessage(String(error), true);
  }
}

function submit(formId, command, fields) {
  const form = $(formId);
  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    const button = form.querySelector('button[type="submit"]');
    const originalLabel = button.textContent;
    button.disabled = true;
    button.textContent = preference("language", "en") === "es" ? "Procesando…" : "Processing…";
    form.setAttribute("aria-busy", "true");
    showMessage("Operación en curso. Espera a que termine.");
    try {
      const args = Object.fromEntries(fields.map(([key, id]) => [key, $(id).value]));
      showMessage(await invoke(command, args));
    } catch (error) {
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

submit("create-form", "create_vault", [["path", "create-path"], ["password", "create-password"]]);
submit("open-form", "open_vault", [["path", "open-path"], ["password", "open-password"]]);
submit("encrypt-form", "encrypt_file", [["inputPath", "encrypt-path"]]);
submit("decrypt-form", "decrypt_file", [["inputPath", "decrypt-input"], ["outputPath", "decrypt-output"]]);
submit("encrypt-folder-form", "encrypt_folder", [["sourcePath", "encrypt-folder-source"], ["vaultFolderPath", "encrypt-folder-output"]]);
submit("decrypt-folder-form", "decrypt_folder", [["vaultFolderPath", "decrypt-folder-source"], ["destinationPath", "decrypt-folder-output"]]);
submit("backup-form", "export_backup", [["backupPath", "backup-path"], ["backupPassphrase", "backup-password"]]);
submit("restore-form", "import_backup", [["backupPath", "restore-backup-path"], ["backupPassphrase", "restore-backup-password"], ["newVaultPath", "restore-vault-path"], ["newVaultPassword", "restore-vault-password"]]);

bindSecretToggle("create-password-toggle", "create-password");
bindSecretToggle("open-password-toggle", "open-password");
bindSecretToggle("backup-password-toggle", "backup-password");
bindSecretToggle("restore-backup-password-toggle", "restore-backup-password");
bindSecretToggle("restore-vault-password-toggle", "restore-vault-password");

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
    document.querySelectorAll(".secret-row").forEach((row) => {
      const input = row.querySelector("input");
      const toggle = row.querySelector("button");
      input.value = "";
      input.type = "password";
      toggle.textContent = preference("language", "en") === "es" ? "Mostrar" : "Show";
      toggle.setAttribute("aria-pressed", "false");
      toggle.setAttribute("aria-label", preference("language", "en") === "es" ? "Mostrar contraseña" : "Show password");
    });
    lockButton.textContent = originalLabel;
    await refreshStatus();
  }
});

refreshStatus();

// Local preferences: language, appearance and profile never leave this device.
const preference = (key, fallback = "") => { try { return localStorage.getItem(`nqv.${key}`) ?? fallback; } catch { return fallback; } };
const storePreference = (key, value) => { try { localStorage.setItem(`nqv.${key}`, value); } catch { /* storage may be unavailable */ } };
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
  "WORKSPACE":"ESPACIO DE TRABAJO", "Overview":"Resumen", "Vault access":"Acceso al vault", "File operations":"Operaciones con archivos", "Settings":"Ajustes", "Help & guide":"Ayuda y guía", "OPEN SOURCE":"CÓDIGO ABIERTO", "Build security with us.":"Construyamos seguridad juntos.", "Explore the project, contribute ideas and leave a star on GitHub.":"Explora el proyecto, colabora y deja una estrella en GitHub.", "View NEXUS-Q repository ↗":"Ver repositorio NEXUS-Q ↗", "Local user":"Usuario local", "LOCAL PROFILE":"PERFIL LOCAL", "LOCAL-FIRST SECURITY":"SEGURIDAD LOCAL", "Light mode":"Modo claro", "Dark mode":"Modo oscuro",
  "PREFERENCES":"PREFERENCIAS", "Settings & profile":"Ajustes y perfil", "Development build · v0.1.0":"Versión de desarrollo · v0.1.0", "Local profile":"Perfil local", "Your profile stays on this device and is not uploaded.":"Tu perfil permanece en este dispositivo y no se sube a ningún servidor.", "Display name":"Nombre visible", "Profile photo":"Foto de perfil", "Stored locally in this app.":"Guardada localmente en esta aplicación.", "Save profile":"Guardar perfil", "Appearance":"Apariencia", "Make Vault feel like your workspace.":"Adapta Vault a tu espacio de trabajo.", "Accent color":"Color de acento", "Typography":"Tipografía", "System minimal":"Sistema minimalista", "Elegant serif":"Serif elegante", "Monospace":"Monoespaciada", "The default palette is monochrome. Your accent and font choices are saved locally.":"La paleta predeterminada es monocromática. Tus colores y tipografía se guardan localmente.", "Apply appearance":"Aplicar apariencia", "Application":"Aplicación", "Build information and important limitations.":"Información de versión y limitaciones importantes.", "Version":"Versión", "0.1.0 · Development":"0.1.0 · Desarrollo", "Data location":"Ubicación de los datos", "On this device":"En este dispositivo", "Telemetry":"Telemetría", "Disabled":"Desactivada", "Do not use this development build as the only copy of important data. Keep encrypted payloads and recovery backups in separate safe locations.":"No utilices esta versión de desarrollo como única copia de tus datos importantes. Guarda los archivos cifrados y los backups de recuperación en ubicaciones seguras y separadas.",
  "GETTING STARTED":"PRIMEROS PASOS", "Help & user guide":"Ayuda y guía de usuario", "Create your vault":"Crea tu vault", "Choose a location, create a strong unique password and keep it safe. The vault file stores your protected key material.":"Elige una ubicación, crea una contraseña única y robusta y guárdala de forma segura. El archivo vault almacena tus claves protegidas.", "Unlock your vault":"Desbloquea tu vault", "Select your existing .nqv file and enter its password. Lock the vault whenever you leave your computer.":"Selecciona tu archivo .nqv e introduce su contraseña. Bloquea el vault cuando te alejes del ordenador.", "Protect files and folders":"Protege archivos y carpetas", "Unlock first, then choose a file or folder to encrypt. Original files are not deleted automatically.":"Primero desbloquea el vault y después elige un archivo o carpeta para cifrar. Los originales no se eliminan automáticamente.", "Make a recovery backup":"Crea un backup de recuperación", "Create an encrypted .nqb backup with a separate strong passphrase. Store it separately from your encrypted files.":"Crea un backup .nqb cifrado con una contraseña independiente y robusta. Guárdalo separado de tus archivos cifrados.", "Restore safely":"Restaura de forma segura", "Use the backup passphrase to restore into a new vault. Keep the encrypted .nqx files and folder-vault data separately.":"Utiliza la contraseña del backup para restaurar en un vault nuevo. Conserva por separado los archivos .nqx y los datos de las carpetas vault.", "Important limitations":"Limitaciones importantes", "Folder encryption currently supports files up to 32 MiB, skips empty folders and rejects symbolic links. This is an early development build, not independently audited.":"El cifrado de carpetas admite archivos de hasta 32 MiB, omite carpetas vacías y rechaza enlaces simbólicos. Es una versión temprana de desarrollo y no ha sido auditada de forma independiente.", "Made for privacy":"Diseñado para la privacidad",
  "Browse":"Examinar", "Show":"Mostrar", "Hide":"Ocultar", "Generate strong password":"Generar contraseña segura", "Store your password in a password manager. An encrypted recovery backup can restore vault keys, but it cannot reveal your original password.":"Guarda tu contraseña en un gestor de contraseñas. Un backup cifrado puede restaurar las claves del vault, pero no revelar la contraseña original."
});
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
}
const languageToggle = $("language-toggle");
if (languageToggle) {
  languageToggle.addEventListener("click", () => translatePage(preference("language", "en") === "en" ? "es" : "en"));
  translatePage(preference("language", "en"));
}
function applyTheme(theme) {
  document.body.classList.toggle("dark", theme === "dark");
  const button = $("theme-toggle");
  if (button) button.textContent = preference("language", "en") === "es" ? (theme === "dark" ? "Modo claro" : "Modo oscuro") : (theme === "dark" ? "Light mode" : "Dark mode");
  storePreference("theme", theme);
  if (!preference("accent")) {
    const base = theme === "dark" ? "#ffffff" : "#111111";
    document.documentElement.style.setProperty("--accent", base);
    document.documentElement.style.setProperty("--accent-ink", theme === "dark" ? "#111" : "#fff");
    if ($("accent-color")) $("accent-color").value = base;
  }
}
applyTheme(preference("theme", "light"));
$("theme-toggle")?.addEventListener("click", () => applyTheme(document.body.classList.contains("dark") ? "light" : "dark"));
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
function applyAppearance() {
  const accent = $("accent-color").value;
  const font = $("font-choice").value;
  const fonts = {system:'Inter,ui-sans-serif,system-ui,sans-serif',inter:'Inter,ui-sans-serif,sans-serif',serif:'Georgia,"Times New Roman",serif',mono:'ui-monospace,SFMono-Regular,monospace'};
  document.documentElement.style.setProperty("--accent", accent);
  document.documentElement.style.setProperty("--accent-ink", ["#ffffff","#fff","#FFFFFF"].includes(accent) ? "#111" : "#fff");
  document.documentElement.style.fontFamily = fonts[font] || fonts.system;
  storePreference("accent", accent); storePreference("font", font);
}
$("accent-color").value = preference("accent", document.body.classList.contains("dark") ? "#ffffff" : "#111111");
$("font-choice").value = preference("font", "system");
applyAppearance();
$("save-appearance")?.addEventListener("click", () => { applyAppearance(); showMessage("Appearance preferences applied and saved locally."); });
document.querySelectorAll(".side-nav a").forEach((link) => link.addEventListener("click", () => {
  document.querySelectorAll(".side-nav a").forEach((item) => item.classList.toggle("active", item === link));
}));

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
