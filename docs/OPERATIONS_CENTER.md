# Operations center

The desktop operations center reports lifecycle events emitted by Rust. It does not estimate progress with frontend timers.

## Lifecycle

Each long-running request receives a random operation ID and moves through `queued`, `running`, and one terminal state: `completed`, `failed`, or `cancelled`. The existing single-session mutex serializes work that uses the unlocked vault; requests waiting for that mutex remain queued. Cancellation requests use a separate registry, so they can be received while an operation owns the session mutex.

Progress updates include a backend phase. A completed/total pair is included only when the backend can measure those units accurately:

- File operations report the completed input-read and output-write byte counts. The current envelope implementation still processes the cryptographic payload in memory, so encryption/decryption phases are indeterminate rather than fabricated percentages.
- Folder operations report completed files out of the validated file count. Each file remains an indivisible in-memory Core operation.
- Backup operations report input/output byte counts where measurable and indeterminate phases for KDF, serialization, and authenticated encryption.

## Cancellation and output safety

Cancellation is cooperative. It is checked at documented safe points and does not interrupt an active cryptographic primitive. For file operations, cancellation before publication removes the temporary output. Folder encryption/restoration removes its incomplete destination tree on cancellation or failure; original source files and encrypted inputs are preserved. Backup export publishes with create-only semantics. Backup import accepts cancellation before destination-vault creation; once creation starts, it completes or removes the newly created vault file on failure.

All output publication is create-only: an existing destination is never silently overwritten. The UI displays an indeterminate progress bar whenever the backend has not supplied a reliable completed/total pair.

Operation events contain only the operation ID, lifecycle status, phase, and numeric progress. They do not include file paths, document names, passphrases, keys, or file contents.
