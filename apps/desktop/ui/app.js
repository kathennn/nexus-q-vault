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
    toggle.textContent = "Mostrar";
    toggle.setAttribute("aria-pressed", "false");
    toggle.setAttribute("aria-label", "Mostrar contraseña");
  });
}

function bindSecretToggle(buttonId, inputId) {
  const button = $(buttonId);
  const input = $(inputId);
  button.addEventListener("click", () => {
    const reveal = input.type === "password";
    input.type = reveal ? "text" : "password";
    button.textContent = reveal ? "Ocultar" : "Mostrar";
    button.setAttribute("aria-pressed", String(reveal));
    button.setAttribute("aria-label", `${reveal ? "Ocultar" : "Mostrar"} contraseña`);
  });
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
    const originalLabel = button.textContent;
    button.disabled = true;
    button.textContent = "Procesando…";
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
    button.textContent = "Abriendo…";
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
  lockButton.textContent = "Bloqueando…";
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
      toggle.textContent = "Mostrar";
      toggle.setAttribute("aria-pressed", "false");
      toggle.setAttribute("aria-label", "Mostrar contraseña");
    });
    lockButton.textContent = originalLabel;
    await refreshStatus();
  }
});

refreshStatus();
