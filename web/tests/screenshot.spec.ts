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
      viewport: { width: 800, height: 1200 },
    });
    const page = context.pages()[0] ?? (await context.newPage());
    await use(page);
    await context.close();
  },
});

test("screenshot connected page", async ({ devicePage: page }) => {
  await page.goto("http://localhost:5173/logi-re/");

  // Try connectGranted first.
  const connected = await page.evaluate(async () => {
    const mod = await import("/logi-re/pkg/hidpp_web.js");
    await mod.default();
    const device = await mod.WasmDevice.connectGranted();
    if (!device) return false;
    (window as any).__device = device;
    return true;
  });

  if (!connected) {
    // Fall back to manual connect.
    await page.click("button:has-text('Connect Device')");
    await page.waitForFunction(
      () => document.querySelector(".connection-badge") !== null,
      { timeout: 30_000 },
    );
    await page.waitForTimeout(3000);
  } else {
    // Trigger the UI manually since we bypassed the button.
    await page.click("button:has-text('Connect Device')");
    await page.waitForFunction(
      () => document.querySelector(".connection-badge") !== null,
      { timeout: 30_000 },
    );
    await page.waitForTimeout(3000);
  }

  // Wait for data to load.
  await page.waitForTimeout(2000);

  // Take screenshot.
  await page.screenshot({
    path: "test-results/connected-page.png",
    fullPage: true,
  });

  console.log("Screenshot saved to test-results/connected-page.png");
});
