const UI_HTML: &str = include_str!("../apps/desktop/ui/index.html");
const UI_JS: &str = include_str!("../apps/desktop/ui/app.js");

#[test]
fn export_and_restore_forms_show_accessible_backup_warnings() {
    let export_form = UI_HTML
        .split("<form id=\"backup-form\"")
        .nth(1)
        .expect("backup export form exists")
        .split("</form>")
        .next()
        .expect("backup export form closes");
    let restore_form = UI_HTML
        .split("<form id=\"restore-form\"")
        .nth(1)
        .expect("backup restore form exists")
        .split("</form>")
        .next()
        .expect("backup restore form closes");

    assert!(export_form.contains("id=\"backup-export-warning\""));
    assert!(export_form.contains("role=\"note\""));
    assert!(export_form.contains(".nqb"));
    assert!(export_form.contains(".nqx"));
    assert!(export_form.contains("manifest.nqm"));

    assert!(restore_form.contains("id=\"backup-restore-warning\""));
    assert!(restore_form.contains("role=\"note\""));
    assert!(restore_form.contains("contraseña independiente"));
    assert!(restore_form.contains("vault nuevo"));
    assert!(restore_form.contains("contraseña original"));
}

#[test]
fn backup_warning_copy_has_english_and_spanish_translations() {
    for english_copy in [
        "Create vault-key backup",
        "The .nqb file protects vault state and keys; it is not a copy of your documents.",
        "What this backup contains",
        "Encrypted vault state and the key material needed to recover it.",
        "Individual .nqx files, manifest.nqm files or encrypted folder objects.",
        "Before restoring: you need both backups",
        "The import creates a new vault protected by the new password you choose here.",
        "The .nqb does not contain encrypted documents and cannot reveal, recover or reset the original vault password.",
        "Make a vault-key backup",
        "Restore keys and documents",
    ] {
        assert!(
            UI_JS.contains(&format!("\"{english_copy}\":")),
            "missing English translation: {english_copy}"
        );
    }

    for spanish_copy in [
        "Crear backup de claves del vault",
        "Qué guarda este backup",
        "Antes de restaurar: necesitas ambos backups",
        "La contraseña del backup es independiente",
        "no revela, recupera ni restablece la contraseña original",
        "Restaura las claves y los documentos",
    ] {
        assert!(
            UI_HTML.contains(spanish_copy) || UI_JS.contains(spanish_copy),
            "missing Spanish copy: {spanish_copy}"
        );
    }
}
