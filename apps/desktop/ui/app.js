const invoke = window.__TAURI__.core.invoke;
const $ = (id) => document.getElementById(id);
const feedback = $("feedback");
const statusLabel = $("vault-status");
const statusDot = $("status-dot");
const lockButton = $("lock-button");
function showMessage(message, isError = false) {
  feedback.hidden = false; feedback.classList.toggle("error", isError); feedback.textContent = message;
}
function clearSecrets(form) { form.querySelectorAll('input[type="password"]').forEach((input) => { input.value = ""; }); }
async function refreshStatus() {
  try {
    const unlocked = await invoke("vault_status");
    statusLabel.textContent = unlocked ? "Desbloqueado" : "Bloqueado";
    statusDot.classList.toggle("active", unlocked); lockButton.disabled = !unlocked;
    document.querySelectorAll(".workspace input,.workspace button[type=submit]").forEach((el) => { el.disabled = !unlocked; });
  } catch (error) { showMessage(String(error), true); }
}
function submit(formId, command, fields) {
  const form = $(formId);
  form.addEventListener("submit", async (event) => {
    event.preventDefault(); const button = form.querySelector('button[type="submit"]'); button.disabled = true;
    try {
      const args = Object.fromEntries(fields.map(([key, id]) => [key, $(id).value]));
      showMessage(await invoke(command, args)); clearSecrets(form); await refreshStatus();
    } catch (error) { showMessage(String(error), true); }
    finally { button.disabled = false; await refreshStatus(); }
  });
}
submit("create-form","create_vault",[["path","create-path"],["password","create-password"]]);
submit("open-form","open_vault",[["path","open-path"],["password","open-password"]]);
submit("encrypt-form","encrypt_file",[["inputPath","encrypt-path"]]);
submit("decrypt-form","decrypt_file",[["inputPath","decrypt-input"],["outputPath","decrypt-output"]]);
submit("backup-form","export_backup",[["backupPath","backup-path"],["backupPassphrase","backup-password"]]);
submit("restore-form","import_backup",[["backupPath","restore-backup-path"],["backupPassphrase","restore-backup-password"],["newVaultPath","restore-vault-path"],["newVaultPassword","restore-vault-password"]]);
lockButton.addEventListener("click", async () => {
  lockButton.disabled = true;
  try { showMessage(await invoke("lock_vault")); }
  catch (error) { showMessage(String(error), true); }
  finally { await refreshStatus(); }
});
refreshStatus();
