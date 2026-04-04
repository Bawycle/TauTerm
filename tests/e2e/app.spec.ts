// SPDX-License-Identifier: MPL-2.0
import { browser, $ } from "@wdio/globals";

describe("TauTerm — application launch", () => {
  it("opens a window with the correct title", async () => {
    const title = await browser.getTitle();
    expect(title).toBe("tau-term");
  });

  it("renders the main terminal view", async () => {
    // +page.svelte mounts a <main class="container"> at the top level.
    const main = await $("main.container");
    await expect(main).toExist();
  });
});
