import { connect, isSupported, type Device } from "./device";
import { qs, el, card, badge, setText } from "./dom";
import { log, logError } from "./log";
import { createConnectScreen } from "./components/connect";
import { createDpiCard } from "./components/dpi";
import { createSmartShiftCard } from "./components/smart-shift";
import { createHostCard } from "./components/host";
import { createFeaturesCard } from "./components/features";

log("HID++ Configurator starting...");

const app = qs("#app") as HTMLDivElement;

const connectScreen = createConnectScreen({
  supported: isSupported(),
  onConnect: () =>
    void (async (): Promise<void> => {
      try {
        log("User clicked Connect...");
        const device = await connect();
        log("Device connected, building UI...");
        await showDeviceUI(device);
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        logError(msg);
        const status = app.querySelector(".status");
        if (status) status.textContent = `Error: ${msg}`;
        const btn = app.querySelector("button");
        if (btn) btn.removeAttribute("disabled");
      }
    })(),
});

app.append(connectScreen);
log("Ready. Click Connect to begin.");

async function showDeviceUI(device: Device): Promise<void> {
  app.replaceChildren();

  // Header with connection badge.
  const header = el(
    "div",
    { class: "header" },
    el("h1", {}, "HID++ Configurator"),
    el(
      "div",
      { class: "connection-badge" },
      el("span", { class: "dot" }),
      el("span", {}, device.name),
    ),
  );
  app.append(header);

  // Stats grid — battery, DPI, mode at a glance.
  const batteryVal = el("span", { class: "stat-value" }, "—");
  const dpiVal = el("span", { class: "stat-value" }, "—");
  const modeVal = el("span", { class: "stat-value" }, "—");
  const hostVal = el("span", { class: "stat-value" }, "—");

  const statsGrid = el(
    "div",
    { class: "stats-grid" },
    el("div", { class: "stat-card" }, batteryVal, el("div", { class: "stat-label" }, "Battery")),
    el("div", { class: "stat-card" }, dpiVal, el("div", { class: "stat-label" }, "DPI")),
    el("div", { class: "stat-card" }, modeVal, el("div", { class: "stat-label" }, "Scroll Mode")),
    el("div", { class: "stat-card" }, hostVal, el("div", { class: "stat-label" }, "Host")),
  );
  app.append(statsGrid);

  // DPI control.
  const dpi = createDpiCard();
  app.append(dpi.root);

  // SmartShift control.
  const smartShift = createSmartShiftCard();
  app.append(smartShift.root);

  // Buttons list.
  const buttonsCard = await createButtonsCard(device);
  if (buttonsCard) app.append(buttonsCard);

  // Firmware info.
  const fwCard = await createFirmwareCard(device);
  if (fwCard) app.append(fwCard);

  // Easy-Switch.
  const host = createHostCard();
  app.append(host.root);

  // Features list (collapsible).
  const features = createFeaturesCard(device.getFeatures());
  app.append(features.root);

  // Read all values and populate stats.
  try {
    const bat = await device.getBattery();
    setText(batteryVal, `${bat.percentage}%`);
  } catch {
    setText(batteryVal, "—");
  }

  try {
    const d = await device.getDpi();
    setText(dpiVal, String(d));
  } catch {
    setText(dpiVal, "—");
  }

  try {
    const ss = await device.getSmartShift();
    setText(modeVal, ss.mode);
  } catch {
    setText(modeVal, "—");
  }

  try {
    const h = await device.getHostInfo();
    setText(hostVal, `${h.currentHost + 1}/${h.numHosts}`);
  } catch {
    setText(hostVal, "—");
  }

  // Refresh detailed cards.
  void dpi.refresh(device).catch((e: unknown) => {
    logError(`dpi: ${String(e)}`);
  });
  void smartShift.refresh(device).catch((e: unknown) => {
    logError(`smartshift: ${String(e)}`);
  });
  void host.refresh(device).catch((e: unknown) => {
    logError(`host: ${String(e)}`);
  });

  log("UI ready.");
}

async function createButtonsCard(device: Device): Promise<HTMLElement | null> {
  try {
    const buttons = await device.getButtons();
    if (buttons.length === 0) return null;

    const NAMES: Record<number, string> = {
      50: "Left Click",
      51: "Right Click",
      80: "Left Click",
      81: "Right Click",
      82: "Middle Click",
      83: "Back",
      86: "Forward",
      195: "Gesture",
      196: "Mode Shift",
      215: "Thumbwheel Click",
    };

    const content = el("div", {});
    for (const btn of buttons) {
      const b = btn as Record<string, number | boolean>;
      const cid = b.cid as number;
      const name = NAMES[cid] ?? `CID ${cid}`;
      const r = el(
        "div",
        { class: "button-row" },
        el("span", { class: "cid" }, `0x${cid.toString(16).padStart(4, "0")}`),
        el("span", { class: "name" }, name),
        b.divertable === true ? badge("divertable", "info") : el("span", {}),
      );
      content.append(r);
    }

    return card("Buttons", content);
  } catch {
    return null;
  }
}

async function createFirmwareCard(device: Device): Promise<HTMLElement | null> {
  try {
    const fw = await device.getFirmware();
    if (fw.length === 0) return null;

    const content = el("div", {});
    for (const ent of fw) {
      const e = ent as Record<string, string | number>;
      const r = el(
        "div",
        { class: "firmware-row" },
        el("span", { class: "fw-name" }, String(e.name)),
        el("span", { class: "fw-type" }, String(e.type)),
        el(
          "span",
          { class: "fw-version" },
          `v${(e.versionMajor as number).toString(16)}.${(e.versionMinor as number).toString(16).padStart(2, "0")} build ${e.build}`,
        ),
      );
      content.append(r);
    }

    return card("Firmware", content);
  } catch {
    return null;
  }
}
