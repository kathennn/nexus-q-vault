"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { createConfirmedFormSubmit } = require("./password-confirmation.js");

const flows = [
  { name: "vault creation", command: "create_vault", fields: [{ value: "vault password", confirmation: "vault password" }] },
  { name: "backup export", command: "export_backup", fields: [{ value: "backup passphrase", confirmation: "backup passphrase" }] },
  { name: "backup restoration", command: "import_backup", fields: [{ value: "new vault password", confirmation: "new vault password" }] },
];

for (const flow of flows) {
  test(`${flow.name} form submits to its Tauri command when confirmation matches`, async () => {
    let invocations = 0;
    let prevented = false;
    const submit = createConfirmedFormSubmit({
      readConfirmations: () => flow.fields,
      onMismatch: () => assert.fail("A matching form must not report a mismatch"),
      onMatch: () => { invocations += 1; return flow.command; },
    });
    const result = submit({ preventDefault() { prevented = true; } });
    assert.equal(prevented, true);
    assert.equal(result.matched, true);
    assert.equal(result.result, flow.command);
    assert.equal(invocations, 1);
  });

  test(`${flow.name} form never invokes Tauri when confirmation mismatches`, () => {
    let invocations = 0;
    let invalidField;
    const mismatched = flow.fields.map((field) => ({ ...field, confirmation: `${field.confirmation}x` }));
    const submit = createConfirmedFormSubmit({
      readConfirmations: () => mismatched,
      onMismatch: (field, index) => { invalidField = { field, index }; },
      onMatch: () => { invocations += 1; },
    });
    const result = submit({ preventDefault() {} });
    assert.equal(result.matched, false);
    assert.equal(result.mismatchIndex, 0);
    assert.equal(invalidField.index, 0);
    assert.equal(invocations, 0);
  });
}

test("confirmation comparison preserves whitespace exactly", () => {
  let invocations = 0;
  const submit = createConfirmedFormSubmit({
    readConfirmations: () => [{ value: " password", confirmation: "password" }],
    onMismatch() {},
    onMatch() { invocations += 1; },
  });
  assert.equal(submit({ preventDefault() {} }).matched, false);
  assert.equal(invocations, 0);
});

test("multiple confirmation fields report the first mismatch", () => {
  const submit = createConfirmedFormSubmit({
    readConfirmations: () => [
      { value: "first", confirmation: "first" },
      { value: "second", confirmation: "different" },
    ],
    onMismatch(_field, index) { assert.equal(index, 1); },
    onMatch() { assert.fail("The command must not run"); },
  });
  assert.deepEqual(submit({ preventDefault() {} }), { matched: false, mismatchIndex: 1 });
});
