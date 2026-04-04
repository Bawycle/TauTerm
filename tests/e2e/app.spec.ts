// SPDX-License-Identifier: MPL-2.0
import { browser, $ } from "@wdio/globals";

describe("TauTerm — application launch", () => {
  it("opens a window with the correct title", async () => {
    const title = await browser.getTitle();
    expect(title).toBe("tau-term");
  });

  it("renders the main terminal view", async () => {
    // +page.svelte mounts .app-shell > TerminalView at the top level.
    const appShell = await $(".app-shell");
    await expect(appShell).toExist();
  });
});
