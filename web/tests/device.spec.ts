import { test as base, expect, type BrowserContext, type Page } from "@playwright/test";
import { chromium } from "@playwright/test";
import { resolve } from "node:path";
import { homedir } from "node:os";

// Persistent Chrome profile — remembers HID permissions across runs.
const PROFILE_DIR = resolve(homedir(), ".config/hidpp-test-chrome");

// Custom fixture that uses a persistent browser context.
const test = base.extend<{ devicePage: Page }>({
  devicePage: async ({}, use) => {
    const context = await chromium.launchPersistentContext(PROFILE_DIR, {
      channel: "chrome",
      headless: false,
      args: [
        "--enable-experimental-web-platform-features",
        "--disable-features=WebHidBlocklist",
      ],
    });
    const page = context.pages()[0] ?? await context.newPage();
    await use(page);
    await context.close();
  },
});

/** Try connectGranted first, fall back to connect (shows picker on first run). */
async function connectDevice(page: Page): Promise<boolean> {
  return page.evaluate(async () => {
    const mod = await import("/hidpp/pkg/hidpp_web.js");
    await mod.default(); // init WASM

    // Try already-granted device first (no gesture needed).
    const granted = await mod.WasmDevice.connectGranted();
    if (granted) {
      (window as any).__device = granted;
      return true;
    }
    return false; // Need manual grant — first run only
  });
}

test.describe("HID++ Device Tests", () => {
  test("connect and read device info", async ({ devicePage: page }) => {
    await page.goto("http://localhost:5173/hidpp/");

    let connected = await connectDevice(page);

    if (!connected) {
      // First run — need to click Connect button for the picker.
      console.log("No granted device. Click Connect and select your mouse in the picker...");
      await page.click("button:has-text('Connect Device')");

      // Wait for the user to select the device in the picker (up to 30s).
      await page.waitForFunction(
        () => document.querySelector(".card h2")?.textContent === "Device",
        { timeout: 30_000 },
      );

      // Now reconnect via the WASM API for our tests.
      connected = await connectDevice(page);
    }

    expect(connected).toBe(true);

    // Read device name.
    const name = await page.evaluate(() => (window as any).__device.name);
    console.log(`Device: ${name}`);
    expect(name).toContain("MX Master 3S");

    // Read feature count.
    const featureCount = await page.evaluate(() => (window as any).__device.featureCount);
    console.log(`Features: ${featureCount}`);
    expect(featureCount).toBeGreaterThan(30);
  });

  test("read battery status", async ({ devicePage: page }) => {
    await page.goto("http://localhost:5173/hidpp/");
    const connected = await connectDevice(page);
    test.skip(!connected, "No granted device — run the connect test first");

    const battery = await page.evaluate(async () => {
      return await (window as any).__device.getBattery();
    });

    console.log(`Battery: ${battery.percentage}% (${battery.level}, ${battery.charging})`);
    expect(battery.percentage).toBeGreaterThanOrEqual(0);
    expect(battery.percentage).toBeLessThanOrEqual(100);
    expect(battery.level).toBeDefined();
    expect(battery.charging).toBeDefined();
  });

  test("read and verify DPI", async ({ devicePage: page }) => {
    await page.goto("http://localhost:5173/hidpp/");
    const connected = await connectDevice(page);
    test.skip(!connected, "No granted device");

    const dpi = await page.evaluate(async () => {
      return await (window as any).__device.getDpi();
    });

    console.log(`DPI: ${dpi}`);
    expect(dpi).toBeGreaterThanOrEqual(200);
    expect(dpi).toBeLessThanOrEqual(8000);
  });

  test("set DPI to 1600 and restore", async ({ devicePage: page }) => {
    await page.goto("http://localhost:5173/hidpp/");
    const connected = await connectDevice(page);
    test.skip(!connected, "No granted device");

    const result = await page.evaluate(async () => {
      const device = (window as any).__device;
      const original = await device.getDpi();
      const applied = await device.setDpi(1600);
      const readBack = await device.getDpi();
      await device.setDpi(original); // Restore
      const restored = await device.getDpi();
      return { original, applied, readBack, restored };
    });

    console.log(`DPI: ${result.original} → ${result.applied} → restored to ${result.restored}`);
    expect(result.applied).toBe(1600);
    expect(result.readBack).toBe(1600);
    expect(result.restored).toBe(result.original);
  });

  test("read SmartShift state", async ({ devicePage: page }) => {
    await page.goto("http://localhost:5173/hidpp/");
    const connected = await connectDevice(page);
    test.skip(!connected, "No granted device");

    const state = await page.evaluate(async () => {
      return await (window as any).__device.getSmartShift();
    });

    console.log(`SmartShift: ${state.mode} (disengage=${state.autoDisengage}, torque=${state.torque})`);
    expect(state.mode).toBeDefined();
    expect(["Ratchet", "FreeScroll"]).toContain(state.mode);
  });

  test("toggle SmartShift free/ratchet and restore", async ({ devicePage: page }) => {
    await page.goto("http://localhost:5173/hidpp/");
    const connected = await connectDevice(page);
    test.skip(!connected, "No granted device");

    const result = await page.evaluate(async () => {
      const device = (window as any).__device;
      const original = await device.getSmartShift();

      // Toggle to opposite mode, keeping current disengage/torque values.
      const targetMode = original.mode === "Ratchet" ? "freespin" : "ratchet";
      const toggled = await device.setSmartShift(targetMode, original.autoDisengage, original.torque);

      // Read back.
      const readBack = await device.getSmartShift();

      // Restore original.
      const restoreMode = original.mode === "Ratchet" ? "ratchet" : "freespin";
      await device.setSmartShift(restoreMode, original.autoDisengage, original.torque);
      const restored = await device.getSmartShift();

      return { original: original.mode, toggled: toggled.mode, readBack: readBack.mode, restored: restored.mode };
    });

    console.log(`SmartShift: ${result.original} → ${result.toggled} → restored to ${result.restored}`);
    expect(result.toggled).not.toBe(result.original);
    expect(result.readBack).toBe(result.toggled);
    expect(result.restored).toBe(result.original);
  });

  test("read Easy-Switch host info", async ({ devicePage: page }) => {
    await page.goto("http://localhost:5173/hidpp/");
    const connected = await connectDevice(page);
    test.skip(!connected, "No granted device");

    const host = await page.evaluate(async () => {
      return await (window as any).__device.getHostInfo();
    });

    console.log(`Easy-Switch: host ${host.currentHost + 1} of ${host.numHosts}`);
    expect(host.numHosts).toBe(3);
    expect(host.currentHost).toBeGreaterThanOrEqual(0);
    expect(host.currentHost).toBeLessThan(host.numHosts);
  });
});
