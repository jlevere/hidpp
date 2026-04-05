import { test as base, type Page } from "@playwright/test";
import { chromium } from "@playwright/test";
import { resolve } from "node:path";
import { homedir } from "node:os";

const PROFILE_DIR = resolve(homedir(), ".config/hidpp-test-chrome");

const test = base.extend<{ devicePage: Page }>({
  devicePage: async ({}, use) => {
    const context = await chromium.launchPersistentContext(PROFILE_DIR, {
      channel: "chrome",
      headless: false,
      args: [
        "--enable-experimental-web-platform-features",
        "--disable-features=WebHidBlocklist",
      ],
      viewport: { width: 1024, height: 768 },
    });
    const page = context.pages()[0] ?? (await context.newPage());
    await use(page);
    await context.close();
  },
});

test("screenshot connected page", async ({ devicePage: page }) => {
  await page.goto("http://localhost:5173/logi-re/");

  // Try auto-connect first.
  const connected = await page.evaluate(async () => {
    const mod = await import("/logi-re/pkg/hidpp_web.js");
    await mod.default();
    const device = await mod.WasmDevice.connectGranted();
    return device !== null && device !== undefined;
  });

  if (!connected) {
    // Manual connect.
    await page.click("button:has-text('Connect Device')");
  }

  // Wait for the device page to fully load.
  await page.waitForSelector(".header, .dpi-value, .section", { timeout: 30_000 });
  await page.waitForTimeout(2000);

  await page.screenshot({
    path: "test-results/connected-page.png",
    fullPage: true,
  });
  console.log("Screenshot saved.");
});
