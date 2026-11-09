# Linux desktop blank-window diagnostic (2026-11-09)

## Symptoms

- NEXUS-Q Vault opened on Linux, but the WebKitGTK window stayed blank/grey.
- The `WebKitWebProcess` became CPU-bound (approximately one full CPU core).
- The affected test machine used Intel Alder Lake-N / UHD Graphics, Mesa 25.2.8, Wayland, and WebKitGTK 2.52.6.
- The same user reported that the published GitHub release rendered correctly.

## Diagnosis and successful workaround

The host had an unusually large Fontconfig catalogue: `fc-list` reported 2,172 distinct font families. A temporary Fontconfig configuration restricted WebKit's visible catalogue to seven DejaVu families. With that reduced catalogue, the Vault frontend rendered successfully.

This is a strong reproduction-specific indication of a WebKitGTK/Fontconfig font-enumeration or font-matching problem. It is not proof that every blank WebKit window has this cause.

The temporary diagnostic files were kept under `/tmp`:

- `/tmp/nexus-vault-fontconfig-minimal.conf`
- `/tmp/nexus-vault-fontconfig-cache/`

They do not modify or delete the user's installed fonts. To repeat the diagnostic, launch the desktop binary with:

```sh
FONTCONFIG_FILE=/tmp/nexus-vault-fontconfig-minimal.conf ./target/release/nexus-q-vault-desktop
```

Do not make this restricted font catalogue a permanent application default without testing typography and fallback for other scripts/locales. Prefer first checking WebKitGTK, Fontconfig and Mesa versions, process CPU, and the system font catalogue.

## Related frontend permission error

Once the UI rendered, the feedback banner showed:

```text
Command plugin:event|listen not allowed by ACL
```

The frontend registers Tauri event listeners (`operation-update`, `vault-auto-locked`, and `vault-lock-failed`). The app had no explicit capability granting the event listener permissions. A default capability was added at `apps/desktop/capabilities/default.json` with `core:event:allow-listen` and `core:event:allow-unlisten`.

## Follow-up validation

- Rebuild the Tauri app after changing capability files.
- Confirm the red ACL banner no longer appears.
- Confirm operation progress and automatic-lock event listeners still work.
- Confirm the frontend renders with the normal system Fontconfig catalogue on the affected machine; the reduced catalogue is diagnostic only.
