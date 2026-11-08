# Automatic vault locking

The desktop application enforces an inactivity timeout in the Rust process. The
default is five minutes; users can select one, five, ten, fifteen or thirty
minutes in Settings, or choose **Never** to disable inactivity locking. The
zero-second disabled value is stored in the local application preferences and
validated again by the Rust command before it is applied. Exit locking remains
active even when inactivity locking is disabled. No activity data is
transmitted or persisted.

Keyboard, pointer, touch and wheel activity is reported to Rust with a
throttled IPC call. Activity updates do not acquire the session mutex, so input
remains observable while a long-running operation is using the session. Rust
owns the deadline and checks it independently of the webview. The watchdog
takes the same session mutex used by cryptographic operations and rechecks the
deadline after acquiring it. An operation already holding the session therefore
completes before the session can be locked, and a new activity update can cancel
a pending timeout.

The vault is also locked when the application exits. When Tauri reports an
event-loop resume, the app attempts to lock any still-open session before
refreshing the inactivity baseline. The frontend clears password fields and
updates the locked status when it receives the lock event. Lock errors are
surfaced to the interface; an exit request is prevented if saving the session
fails. The desktop app uses the engine's `Session::lock_recoverable(self)`
API: if persistence fails, the error returns ownership of the still-unlocked
session, and the app restores it to managed state so the user can retry.
Failed automatic and resume-triggered attempts are throttled for 30 seconds
to avoid repeatedly emitting the same error; new user activity clears that
delay. A failed resume lock does not reset the previous inactivity deadline.
The legacy
`Session::lock(self)` API remains available for existing callers and retains
its previous drop-on-error behavior.

## Platform limitation

Tauri 2 documents its native suspended/resumed lifecycle events as mobile-only;
they are not available on Linux, macOS or Windows. The desktop app therefore
does not claim reliable detection of an operating-system screen lock or sleep
on those platforms. Hiding the webview (for example, minimizing the window)
also requests an immediate lock as a best-effort additional safeguard.
