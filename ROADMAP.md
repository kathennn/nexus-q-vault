# Roadmap

- [x] Foundation: repository structure, license, governance and initial threat model.
- [x] MVP definition and nexusq-core integration spike.
- [x] Public repository and contribution templates.
- [ ] Security-reviewed vault/file/backup adapter and automated integration tests.
- [ ] Recovery design and user-tested recovery flow.
  - [x] Draft backup-based recovery proposal and automated restore with a new vault password (user testing still pending).
- [ ] Encrypted folder manifest and format migration policy.
  - [x] Version-1 encrypted manifest, opaque object names, safe relative paths, atomic create-only publication and folder roundtrip tests.
  - [ ] Format migration, empty-directory preservation, large-file streaming and cross-platform filesystem review.
- [ ] Desktop UI and accessible onboarding.
  - [x] Initial Tauri 2 desktop shell and Rust command boundary (prototype only; not yet user-tested).
  - [x] UI commands for individual files, folder encryption/restoration and encrypted key backups.
  - [x] Native file/folder pickers through Tauri's cross-platform dialog plugin.
  - [x] Initial accessibility pass: semantic status/errors, labelled picker controls, busy states, keyboard focus and reduced-motion support.
  - [ ] User testing, onboarding polish and full accessibility review.
- [ ] CI, dependency review, fuzzing and reproducible-build investigation.
- [ ] External security review and community beta.
- [ ] 1.0 release after recovery, format and update-security criteria are met.

A roadmap item is not complete merely because a prototype exists. Security
milestones require published evidence and review.
