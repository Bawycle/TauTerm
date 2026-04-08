// SPDX-License-Identifier: MPL-2.0

/**
 * usePasteDialog — paste confirmation dialog state sub-composable.
 *
 * Manages pasteConfirmOpen and pasteConfirmText reactive state.
 * openPasteConfirm() sets the text and opens the dialog.
 * closePasteConfirm() clears state and closes the dialog.
 */

export function usePasteDialog() {
  let pasteConfirmOpen = $state(false);
  let pasteConfirmText = $state('');

  function openPasteConfirm(text: string) {
    pasteConfirmText = text;
    pasteConfirmOpen = true;
  }

  function closePasteConfirm() {
    pasteConfirmOpen = false;
    pasteConfirmText = '';
  }

  return {
    get pasteConfirmOpen() {
      return pasteConfirmOpen;
    },
    get pasteConfirmText() {
      return pasteConfirmText;
    },
    openPasteConfirm,
    closePasteConfirm,
  };
}
