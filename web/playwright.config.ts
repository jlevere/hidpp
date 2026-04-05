import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  timeout: 60_000,
  // Tests use custom persistent context fixture — no default browser config needed.
  // The devicePage fixture in device.spec.ts handles Chrome launch with HID flags.
});
