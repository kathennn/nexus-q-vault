const invoke = window.__TAURI__.core.invoke;
const $ = (id) => document.getElementById(id);
const feedback = $("feedback");
const statusLabel = $("vault-status");
const statusDot = $("status-dot");
const lockButton = $("lock-button");

function showMessage(message, isError = false) {
  feedback.hidden = false;
  feedback.classList.toggle("error", isError);
  feedback.textContent = message;
}

function clearSecrets(form) {
  form.querySelectorAll('input[type="password"]').forEach((input) => { input.value = ""; });
}

async function refreshStatus() {
  try {
    const unlocked = await invoke("vault_status");
    statusLabel.textContent = unlocked ? "Desbloqueado" : "Bloqueado";
    statusDot.classList.toggle("active", unlocked);
    lockButton.disabled = !unlocked;
    document.querySelectorAll(".workspace input,.workspace button[type=submit],.workspace .picker")
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
    button.disabled = true;
    try {
      const args = Object.fromEntries(fields.map(([key, id]) => [key, $(id).value]));
      showMessage(await invoke(command, args));
      await refreshStatus();
    } catch (error) {
      showMessage(String(error), true);
    } finally {
      clearSecrets(form);
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
    button.disabled = true;
    try {
      const selectedPath = await invoke(command, { kind });
      if (selectedPath) {
        $(inputId).value = transform(selectedPath);
        showMessage("Ruta seleccionada.");
      }
    } catch (error) {
      showMessage(String(error), true);
    } finally {
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
  lockButton.disabled = true;
  try {
    showMessage(await invoke("lock_vault"));
  } catch (error) {
    showMessage(String(error), true);
  } finally {
    document.querySelectorAll('input[type="password"]').forEach((input) => { input.value = ""; });
    await refreshStatus();
  }
});

refreshStatus();
