# Independent-installation recovery acceptance

The automated `recovery_works_across_independent_processes` integration test exercises the recovery API in two separately spawned processes. Each process receives a different, empty OS/application profile and its own working directory. The source process creates the vault, encrypted document payloads, folder vault and `.nqb` backup, then exits. The restore process starts without the source session or profile, imports the backup into a new vault using a new vault password, and decrypts the separately preserved payloads.

The automated scenario also checks missing, truncated and corrupted backups, an existing destination that must remain untouched, missing and tampered individual-file payloads, and missing and tampered folder objects. Failed restores must not publish partial vaults or plaintext outputs.

This process-level test does not replace acceptance testing of the packaged desktop application. Before a release, perform the following manual check on a clean machine or a separate OS user account for each supported desktop platform:

1. Install the same supported Vault release in profile A. Create a test vault with a unique vault password.
2. Encrypt at least one binary file and a nested folder containing an empty file and a non-ASCII filename. Keep the original test files unchanged.
3. Export a `.nqb` backup with a backup passphrase different from the vault password.
4. Copy the `.nqb` file and the encrypted `.nqx` sidecar plus complete folder-vault directory to separate storage. Do not assume the `.nqb` contains document payloads; it contains vault/key state only.
5. Exit the application. Install or launch the same release in a clean profile B with no access to profile A application state.
6. Import the `.nqb` using its backup passphrase and choose a new vault password. Confirm that the original vault password does not unlock the recovered vault.
7. Restore the separately preserved `.nqx` file and folder-vault directory. Compare the recovered files byte-for-byte with the original test files, including the empty file and non-ASCII filename.
8. On disposable copies, try an incorrect backup passphrase, a truncated or modified `.nqb`, and a modified encrypted payload. Each operation must fail without publishing partial plaintext or replacing the original vault.
9. Confirm that profile A original vault and source files remain unchanged.

Use synthetic test data only. Never use personal documents, real credentials, recovery phrases or production key material. Record the OS, application version and outcome; do not attach backups, encrypted payloads or secrets to an issue or pull request.
