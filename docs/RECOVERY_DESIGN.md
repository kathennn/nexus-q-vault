# Recovery design proposal — backup-based recovery

**Status:** proposed for the MVP; not yet approved by user testing or an external security review.

## Proposed recovery model

Use the existing encrypted backup mechanism as the recovery factor. Do not introduce a recovery phrase or password-reset protocol in the desktop app until `nexusq-core` has a reviewed, versioned implementation and dedicated tests.

A backup is encrypted with a passphrase independent from the vault password. Importing it creates a new vault and re-wraps its key records under a newly chosen vault password. This is recovery by restoration, not a reset of the original vault password.

## User journey

1. Create a vault with a strong, unique vault password.
2. Create an encrypted backup using a different, strong backup passphrase.
3. Store the backup file separately from the device. Store the backup passphrase separately from the backup file.
4. Keep the encrypted `.nqx` file envelopes or complete folder-vault directories as well. The current backup format contains vault/key state, not the encrypted file payloads or folder objects.
5. Test recovery by importing the backup to a new vault path with a new vault password, then decrypting a known test envelope. Do not overwrite or delete the original vault during this test.
6. If the original vault password is lost, import the backup with its backup passphrase into a new vault using a new password, then use the restored keys to decrypt the separately preserved `.nqx` files and restore the encrypted folder-vault directories.

## Failure and security properties

- The backup passphrase is not the vault password and must not be reused as one.
- The backup passphrase must not be stored beside the backup file.
- Losing both the original vault password and the backup passphrase means the vault cannot be recovered through this proposal.
- Losing the backup file means there is no backup-based recovery.
- A backup is not a copy of the encrypted documents in the current prototype; both required recovery components must be preserved: the `.nqb` vault-key backup (and its separate passphrase), plus the separately stored encrypted `.nqx` files and folder-vault payloads. After importing the `.nqb` into a new vault, the restored keys are used to decrypt those payloads.
- Import creates a new vault at a new path. The original vault is left untouched.
- Do not claim secure deletion, password reset, phrase recovery, or protection from malware on an unlocked device.

## Release gates

- [x] Automated tests prove restore with a new vault password, rejection of the original password, and recovery of pre-existing individual-file and folder-vault payloads.
- Add backup verification and clear UI warnings about what is and is not included.
- Test the recovery journey with non-technical users, including a failed/wrong backup passphrase case.
- Review the design against the threat model before beta.

This proposal covers the current individual-file and folder-vault prototypes. The key backup still excludes all encrypted file/folder payloads. Revisit the design if a future multi-file backup format bundles those artifacts.
