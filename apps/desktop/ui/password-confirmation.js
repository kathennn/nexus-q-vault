(function (root) {
  "use strict";
  function runConfirmedCommand(confirmations, command) {
    const mismatchIndex = confirmations.findIndex(
      ({ value, confirmation }) => value !== confirmation,
    );
    if (mismatchIndex !== -1) return { matched: false, mismatchIndex };
    return { matched: true, result: command() };
  }
  root.passwordConfirmation = { runConfirmedCommand };
  if (typeof module !== "undefined" && module.exports) module.exports = root.passwordConfirmation;
})(globalThis);
