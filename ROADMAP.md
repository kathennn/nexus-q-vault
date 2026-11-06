# Roadmap

- [x] Foundation: repository structure, license, governance and initial threat model.
- [x] MVP definition and nexusq-core integration spike.
- [x] Public repository and contribution templates.
- [ ] Security-reviewed vault/file/backup adapter and automated integration tests.
- [ ] Recovery design and user-tested recovery flow.
  - [x] Draft backup-based recovery proposal and automated restore with a new vault password (user testing still pending).
- [ ] Encrypted folder manifest and format migration policy.
- [ ] Desktop UI and accessible onboarding.
  - [x] Initial Tauri 2 desktop shell and Rust command boundary (prototype only; not yet user-tested).
  - [ ] Native file/folder pickers, onboarding polish and accessibility review.
- [ ] CI, dependency review, fuzzing and reproducible-build investigation.
- [ ] External security review and community beta.
- [ ] 1.0 release after recovery, format and update-security criteria are met.

A roadmap item is not complete merely because a prototype exists. Security
milestones require published evidence and review.
