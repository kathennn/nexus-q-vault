(function (root) {
  "use strict";
  function createConfirmedFormSubmit({ readConfirmations, onMismatch, onMatch }) {
    return function handleSubmit(event) {
      event.preventDefault();
      const confirmations = readConfirmations();
      const mismatchIndex = confirmations.findIndex(({ value, confirmation }) => value !== confirmation);
      if (mismatchIndex !== -1) {
        onMismatch(confirmations[mismatchIndex], mismatchIndex);
        return { matched: false, mismatchIndex };
      }
      return { matched: true, result: onMatch() };
    };
  }
  root.passwordConfirmation = { createConfirmedFormSubmit };
  if (typeof module !== "undefined" && module.exports) module.exports = root.passwordConfirmation;
})(globalThis);
