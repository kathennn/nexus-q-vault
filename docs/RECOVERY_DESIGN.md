# Recovery design proposal — backup-based recovery

**Status:** proposed for the MVP; not yet approved by user testing or an external security review.

## Proposed recovery model

Use the existing encrypted backup mechanism as the recovery factor. Do not introduce a recovery phrase or password-reset protocol in the desktop app until `nexusq-core` has a reviewed, versioned implementation and dedicated tests.

A backup is encrypted with a passphrase independent from the vault password. Importing it creates a new vault and re-wraps its key records under a newly chosen vault password. This is recovery by restoration, not a reset of the original vault password.

## User journey

1. Create a vault with a strong, unique vault password.
2. Create an encrypted backup using a different, strong backup passphrase.
3. Store the backup file separately from the device. Store the backup passphrase separately from the backup file.
4. Keep the encrypted `.nqx` file envelopes as well. The current backup format contains vault/key state, not the encrypted file payloads.
5. Test recovery by importing the backup to a new vault path with a new vault password, then decrypting a known test envelope. Do not overwrite or delete the original vault during this test.
6. If the original vault password is lost, import the backup with its backup passphrase into a new vault using a new password, then open the separately preserved `.nqx` files.

## Failure and security properties

- The backup passphrase is not the vault password and must not be reused as one.
- The backup passphrase must not be stored beside the backup file.
- Losing both the original vault password and the backup passphrase means the vault cannot be recovered through this proposal.
- Losing the backup file means there is no backup-based recovery.
- A backup is not a copy of the encrypted documents in the current prototype; both the backup and the `.nqx` envelopes must be preserved.
- Import creates a new vault at a new path. The original vault is left untouched.
- Do not claim secure deletion, password reset, phrase recovery, or protection from malware on an unlocked device.

## Release gates

- Add automated tests proving restore with a new vault password and rejection of the original password on the restored vault.
- Verify that an envelope encrypted before backup can be decrypted after recovery.
- Add backup verification and clear UI warnings about what is and is not included.
- Test the recovery journey with non-technical users, including a failed/wrong backup passphrase case.
- Review the design against the threat model before beta.

This proposal is scoped to the current single-file prototype. It must be revisited when the encrypted folder manifest and multi-file backup format are designed.
