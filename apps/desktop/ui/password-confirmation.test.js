"use strict";
const test = require("node:test");
const assert = require("node:assert/strict");
const { runConfirmedCommand } = require("./password-confirmation.js");
const flows = [
  { name: "vault creation", matching: [{ value: "vault password", confirmation: "vault password" }], mismatching: [{ value: "vault password", confirmation: "vault passw0rd" }] },
  { name: "backup export", matching: [{ value: "backup passphrase", confirmation: "backup passphrase" }], mismatching: [{ value: "backup passphrase", confirmation: "backup passphrases" }] },
  { name: "backup restoration", matching: [{ value: "new vault password", confirmation: "new vault password" }], mismatching: [{ value: "new vault password", confirmation: "different password" }] },
];
for (const flow of flows) {
  test(`${flow.name}: matching confirmation invokes the Tauri command`, () => {
    let count = 0;
    const result = runConfirmedCommand(flow.matching, () => { count += 1; return "invoked"; });
    assert.equal(result.matched, true);
    assert.equal(result.result, "invoked");
    assert.equal(count, 1);
  });
  test(`${flow.name}: mismatched confirmation prevents the Tauri command`, () => {
    let count = 0;
    const result = runConfirmedCommand(flow.mismatching, () => { count += 1; });
    assert.deepEqual(result, { matched: false, mismatchIndex: 0 });
    assert.equal(count, 0);
  });
}
test("confirmation comparison preserves whitespace exactly", () => {
  let count = 0;
  const result = runConfirmedCommand([{ value: " password", confirmation: "password" }], () => { count += 1; });
  assert.equal(result.matched, false);
  assert.equal(count, 0);
});
test("multiple confirmations report the first mismatch", () => {
  const result = runConfirmedCommand([
    { value: "first", confirmation: "first" },
    { value: "second", confirmation: "different" },
  ], () => assert.fail("The command must not run when any confirmation differs"));
  assert.deepEqual(result, { matched: false, mismatchIndex: 1 });
});
