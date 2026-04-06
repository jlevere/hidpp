import { test, type Page } from "@playwright/test";
import { chromium } from "@playwright/test";
import { resolve } from "node:path";
import { homedir } from "node:os";

const PROFILE_DIR = resolve(homedir(), ".config/hidpp-test-chrome");

const testWithBrowser = test.extend<{ devicePage: Page }>({
  devicePage: async ({}, use) => {
    const context = await chromium.launchPersistentContext(PROFILE_DIR, {
      channel: "chrome",
      headless: false,
      args: ["--enable-experimental-web-platform-features"],
      viewport: { width: 1024, height: 768 },
    });
    const page = context.pages()[0] ?? (await context.newPage());
    await use(page);
    await context.close();
  },
});

testWithBrowser("screenshot demo picker", async ({ devicePage: page }) => {
  await page.goto("http://localhost:5173/hidpp/");
  await page.waitForSelector(".browse-link", { timeout: 5000 });
  await page.click(".browse-link");
  await page.waitForSelector(".demo-grid", { timeout: 10000 });
  await page.waitForTimeout(500);
  await page.screenshot({ path: "test-results/demo-picker.png", fullPage: true });
  console.log("Demo picker screenshot saved.");
});

testWithBrowser("screenshot demo mode DPI", async ({ devicePage: page }) => {
  await page.goto("http://localhost:5173/hidpp/");
  await page.waitForSelector(".browse-link", { timeout: 5000 });
  await page.click(".browse-link");
  await page.waitForSelector(".demo-grid", { timeout: 10000 });
  // Click the first mouse device.
  await page.click(".demo-card >> nth=0");
  await page.waitForSelector(".sidebar", { timeout: 5000 });
  await page.waitForTimeout(500);
  await page.screenshot({ path: "test-results/demo-dpi.png" });
  console.log("Demo DPI screenshot saved.");
});
