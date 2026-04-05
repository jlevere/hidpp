import { connect, isSupported, type Device } from "./device";
import { el } from "./dom";
import { log, logError } from "./log";
import init, { WasmDevice } from "hidpp-web";

log("HID++ Configurator starting...");
const app = document.getElementById("app") as HTMLDivElement;

const BUTTON_NAMES: Record<number, string> = {
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
  199: "Dictation",
  200: "Emoji",
  110: "Screen Capture",
};

const HOTSPOTS = [
  { cid: 82, name: "Middle Click", x: 71, y: 15 },
  { cid: 196, name: "Mode Shift", x: 81, y: 34 },
  { cid: 86, name: "Forward", x: 35, y: 43 },
  { cid: 0, name: "Thumbwheel", x: 55, y: 51.5 },
  { cid: 195, name: "Gesture", x: 8, y: 58 },
  { cid: 83, name: "Back", x: 45, y: 60 },
] as const;

const DPI_PRESETS = [400, 800, 1200, 1600, 2400, 3200] as const;

// ── Connect screen ──

function showConnect(): void {
  const root = el("div", { class: "connect" });
  root.append(
    el("h1", {}, "間 HID++"),
    el("p", {}, "Configure Logitech devices from your browser."),
  );

  if (!isSupported()) {
    root.append(
      el("p", { style: "color: var(--error)" }, "WebHID not available. Use Chrome or Edge."),
    );
    app.replaceChildren(root);
    return;
  }

  const status = el("div", { class: "status" });
  const btn = el("button", { class: "primary" }, "Connect");
  btn.addEventListener("click", () => {
    btn.setAttribute("disabled", "");
    status.textContent = "connecting...";
    void (async (): Promise<void> => {
      try {
        const device = await connect();
        showDevice(device);
      } catch (e) {
        status.textContent = String(e);
        btn.removeAttribute("disabled");
      }
    })();
  });

  const browse = el("button", { class: "browse" }, "browse all devices →");
  browse.addEventListener("click", () => {
    void showDemoPicker();
  });

  root.append(btn, status, browse);
  app.replaceChildren(root);
}

// ── Demo picker ──

async function showDemoPicker(): Promise<void> {
  await init();
  const json = WasmDevice.getDeviceCatalog();
  const catalog = (JSON.parse(json) as { devices: Record<string, unknown>[] }).devices;

  const root = el("div", { class: "demo-picker" });
  root.append(el("h2", {}, "Browse devices"));

  const groups = new Map<string, Record<string, unknown>[]>();
  const order = ["MOUSE", "KEYBOARD", "TRACKBALL", "PRESENTER", "TOUCHPAD", "DIAL"];
  const labels: Record<string, string> = {
    MOUSE: "mice",
    KEYBOARD: "keyboards",
    TRACKBALL: "trackballs",
    PRESENTER: "presenters",
    TOUCHPAD: "touchpads",
    DIAL: "dials",
  };

  for (const dev of catalog) {
    const t = dev.type as string;
    if (
      ["RECEIVER", "VIRTUAL_DEVICE", "CAMERA", "ILLUMINATION_LIGHT", "CONTEXTUAL_KEYS"].includes(t)
    )
      continue;
    const arr = groups.get(t) ?? [];
    arr.push(dev);
    groups.set(t, arr);
  }

  for (const type of order) {
    const devs = groups.get(type);
    if (!devs?.length) continue;
    root.append(el("div", { class: "demo-group-label" }, labels[type] ?? type));
    const grid = el("div", { class: "demo-grid" });
    for (const dev of devs) {
      const btn = el("button", {}, dev.displayName as string);
      btn.addEventListener("click", () => {
        showDemoDevice(dev);
      });
      grid.append(btn);
    }
    root.append(grid);
  }

  const back = el("button", { class: "browse", style: "margin-top: 2rem" }, "← back");
  back.addEventListener("click", () => {
    showConnect();
  });
  root.append(back);

  app.replaceChildren(root);
}

// ── Demo device view ──

function showDemoDevice(profile: Record<string, unknown>): void {
  const name = profile.displayName as string;
  const caps = profile.capabilities as Record<string, unknown> | undefined;
  const dpiInfo = caps?.highResolutionSensorInfo as Record<string, number> | undefined;
  const defaultDpi = dpiInfo?.defaultDpiValueSensorOff ?? 1000;

  renderDevicePage({
    name,
    demo: true,
    battery: null,
    dpi: dpiInfo ? defaultDpi : null,
    dpiRange: dpiInfo
      ? {
          min: dpiInfo.minDpiValueSensorOff ?? 200,
          max: dpiInfo.maxDpiValueSensorOff ?? 4000,
          step: dpiInfo.stepsSensorOff ?? 50,
        }
      : null,
    smartShift: (caps?.scroll_wheel_capabilities as Record<string, boolean> | undefined)?.smartshift
      ? "Ratchet"
      : null,
    buttons: (caps?.specialKeys as { programmable?: number[] } | undefined)?.programmable ?? [],
    hosts: (caps?.flow as { hostCount?: number } | undefined)?.hostCount ?? 0,
    firmware: [],
    features: [],
    friendlyName: null,
    device: null,
  });
}

// ── Live device view ──

function showDevice(device: Device): void {
  renderDevicePage({
    name: device.name,
    demo: false,
    battery: null,
    dpi: null,
    dpiRange: null,
    smartShift: null,
    buttons: [],
    hosts: 0,
    firmware: [],
    features: device.getFeatures() as { id: string; name: string }[],
    friendlyName: null,
    device,
  });

  // Load all data asynchronously.
  void loadLiveData(device);
}

interface PageData {
  name: string;
  demo: boolean;
  battery: { percentage: number; level: string; charging: string } | null;
  dpi: number | null;
  dpiRange: { min: number; max: number; step: number } | null;
  smartShift: string | null;
  buttons: number[];
  hosts: number;
  firmware: {
    name: string;
    type: string;
    versionMajor: number;
    versionMinor: number;
    build: number;
  }[];
  features: { id: string; name: string }[];
  friendlyName: string | null;
  device: Device | null;
}

let currentData: PageData | null = null;
let dpiDisplay: HTMLElement | null = null;
let dpiSlider: HTMLInputElement | null = null;
let dpiInput: HTMLInputElement | null = null;
let presetBtns: HTMLButtonElement[] = [];
let smartShiftLabel: HTMLElement | null = null;
let ratchetBtn: HTMLButtonElement | null = null;
let freeBtn: HTMLButtonElement | null = null;

function renderDevicePage(data: PageData): void {
  currentData = data;
  const root = el("div", {});

  // Header.
  const meta = el("span", { class: "meta" });
  if (data.demo) {
    meta.append(el("span", { class: "demo-tag" }, "DEMO"));
  }
  if (data.battery) {
    meta.textContent = `${String(data.battery.percentage)}%`;
  }
  root.append(el("div", { class: "header" }, el("h1", {}, data.name), meta));

  // Device image with hotspots.
  const imgSection = el("div", { class: "device-image" });
  const imgInner = el("div", { class: "device-image-inner" });
  imgInner.append(
    el("img", {
      src: "/logi-re/devices/assets/mx-master-3s/side.png",
      alt: data.name,
      draggable: "false",
    }),
  );
  for (const h of HOTSPOTS) {
    const dot = el("button", {
      class: "hotspot",
      title: h.name,
      style: `left:${String(h.x)}%;top:${String(h.y)}%`,
    });
    dot.addEventListener("click", () => {
      log(`Button: ${h.name}`);
    });
    imgInner.append(dot);
  }
  imgSection.append(imgInner);
  root.append(imgSection);

  // DPI section.
  if (data.dpi !== null || data.dpiRange !== null) {
    const section = el("div", { class: "section" });
    section.append(el("div", { class: "section-label" }, "DPI"));

    dpiDisplay = el("div", { class: "dpi-value" }, data.dpi !== null ? String(data.dpi) : "—");
    section.append(dpiDisplay);

    const range = data.dpiRange ?? { min: 200, max: 8000, step: 50 };
    dpiSlider = el("input", {
      type: "range",
      min: String(range.min),
      max: String(range.max),
      step: String(range.step),
      value: String(data.dpi ?? 1000),
    });
    dpiInput = el("input", {
      type: "number",
      min: String(range.min),
      max: String(range.max),
      step: String(range.step),
      value: String(data.dpi ?? 1000),
    });

    dpiSlider.addEventListener("input", () => {
      if (dpiInput) dpiInput.value = dpiSlider!.value;
    });
    dpiInput.addEventListener("input", () => {
      if (dpiSlider) dpiSlider.value = dpiInput!.value;
    });
    dpiInput.addEventListener("change", () => {
      void applyDpi();
    });

    section.append(el("div", { class: "dpi-controls" }, dpiSlider, dpiInput));

    const presetsDiv = el("div", { class: "dpi-presets" });
    presetBtns = [];
    for (const p of DPI_PRESETS) {
      const btn = el("button", {}, String(p));
      btn.addEventListener("click", () => {
        void applyDpi(p);
      });
      presetBtns.push(btn);
      presetsDiv.append(btn);
    }
    section.append(presetsDiv);
    root.append(section);
  }

  // Scroll section.
  if (data.smartShift !== null) {
    const section = el("div", { class: "section" });
    section.append(el("div", { class: "section-label" }, "Scroll"));

    smartShiftLabel = el("span", { class: "row-value" }, data.smartShift);
    ratchetBtn = el("button", {}, "ratchet");
    freeBtn = el("button", {}, "free");
    if (data.smartShift === "Ratchet") ratchetBtn.classList.add("active");
    else freeBtn.classList.add("active");

    if (data.demo) {
      ratchetBtn.setAttribute("disabled", "");
      freeBtn.setAttribute("disabled", "");
    }
    ratchetBtn.addEventListener("click", () => {
      void setSmartShift("ratchet");
    });
    freeBtn.addEventListener("click", () => {
      void setSmartShift("freespin");
    });

    section.append(
      el(
        "div",
        { class: "row" },
        el("span", { class: "row-label" }, "Mode"),
        el("div", { class: "toggle" }, ratchetBtn, freeBtn),
      ),
    );
    root.append(section);
  }

  // Buttons section.
  if (data.buttons.length > 0) {
    const section = el("div", { class: "section" });
    section.append(el("div", { class: "section-label" }, "Buttons"));
    for (const cid of data.buttons) {
      const name = BUTTON_NAMES[cid] ?? `Control ${String(cid)}`;
      section.append(
        el(
          "div",
          { class: "button-item" },
          el("span", { class: "cid" }, `0x${cid.toString(16).padStart(4, "0")}`),
          el("span", { class: "name" }, name),
        ),
      );
    }
    root.append(section);
  }

  // Host section.
  if (data.hosts > 0) {
    const section = el("div", { class: "section", id: "host-section" });
    section.append(el("div", { class: "section-label" }, "Easy-Switch"));
    for (let i = 0; i < data.hosts; i++) {
      section.append(
        el(
          "div",
          { class: "row" },
          el("span", { class: "row-label" }, `Slot ${String(i + 1)}`),
          el("span", { class: "row-value" }, "—"),
        ),
      );
    }
    root.append(section);
  }

  // Firmware.
  if (data.firmware.length > 0) {
    const section = el("div", { class: "section" });
    section.append(el("div", { class: "section-label" }, "Firmware"));
    for (const fw of data.firmware) {
      section.append(
        el(
          "div",
          { class: "fw-item" },
          el("span", { class: "fw-name" }, fw.name),
          el("span", { class: "fw-type" }, fw.type),
          el(
            "span",
            {},
            `v${fw.versionMajor.toString(16)}.${fw.versionMinor.toString(16).padStart(2, "0")}`,
          ),
        ),
      );
    }
    root.append(section);
  }

  // Features.
  if (data.features.length > 0) {
    const section = el("div", { class: "section" });
    section.append(
      el("div", { class: "section-label" }, `Features · ${String(data.features.length)}`),
    );
    const wrap = el("div", {});
    for (const f of data.features) {
      wrap.append(
        el(
          "span",
          { class: "feature-item" },
          el("span", { class: "fid" }, f.id),
          document.createTextNode(` ${f.name}`),
        ),
      );
    }
    section.append(wrap);
    root.append(section);
  }

  // Actions.
  if (data.device) {
    const section = el("div", { class: "section" });
    const exportBtn = el("button", {}, "export config");
    exportBtn.addEventListener("click", () => {
      void (async (): Promise<void> => {
        if (!data.device) return;
        const toml = await data.device.exportConfig();
        await navigator.clipboard.writeText(toml);
        exportBtn.textContent = "copied";
        setTimeout(() => {
          exportBtn.textContent = "export config";
        }, 2000);
      })();
    });
    section.append(el("div", { class: "actions" }, exportBtn));
    root.append(section);
  }

  app.replaceChildren(root);
  updateDpiDisplay(data.dpi ?? 1000);
}

function updateDpiDisplay(dpi: number): void {
  if (dpiDisplay) dpiDisplay.textContent = String(dpi);
  if (dpiSlider) dpiSlider.value = String(dpi);
  if (dpiInput) dpiInput.value = String(dpi);
  for (const btn of presetBtns) {
    btn.classList.toggle("active", btn.textContent === String(dpi));
  }
}

async function applyDpi(preset?: number): Promise<void> {
  if (!currentData?.device) return;
  const dpi = preset ?? parseInt(dpiInput?.value ?? "1000", 10);
  if (isNaN(dpi)) return;
  try {
    const applied = await currentData.device.setDpi(dpi);
    updateDpiDisplay(applied);
    log(`DPI → ${String(applied)}`);
  } catch (e) {
    logError(`DPI: ${String(e)}`);
  }
}

async function setSmartShift(mode: string): Promise<void> {
  if (!currentData?.device) return;
  try {
    const result = await currentData.device.setSmartShift(mode, 0, 0);
    if (smartShiftLabel) smartShiftLabel.textContent = result.mode;
    ratchetBtn?.classList.toggle("active", result.mode === "Ratchet");
    freeBtn?.classList.toggle("active", result.mode === "FreeScroll");
    log(`SmartShift → ${result.mode}`);
  } catch (e) {
    logError(`SmartShift: ${String(e)}`);
  }
}

async function loadLiveData(device: Device): Promise<void> {
  const data = currentData;
  if (!data) return;

  try {
    const b = await device.getBattery();
    data.battery = b;
    updateHeader();
  } catch {
    /* */
  }
  try {
    const d = await device.getDpi();
    data.dpi = d;
    updateDpiDisplay(d);
  } catch {
    /* */
  }
  try {
    const ss = await device.getSmartShift();
    data.smartShift = ss.mode;
    if (smartShiftLabel) smartShiftLabel.textContent = ss.mode;
    ratchetBtn?.classList.toggle("active", ss.mode === "Ratchet");
    freeBtn?.classList.toggle("active", ss.mode === "FreeScroll");
  } catch {
    /* */
  }
  try {
    const btns = await device.getButtons();
    data.buttons = (btns as { cid: number }[]).map((b) => b.cid);
    // Re-render buttons section.
    const section = document.querySelector("#app .section:has(.button-item)")?.parentElement;
    if (section) {
      /* buttons already rendered from features */
    }
  } catch {
    /* */
  }
  try {
    const fw = await device.getFirmware();
    data.firmware = fw as typeof data.firmware;
    // Need to re-render to show firmware — for now just log.
    log(
      `Firmware: ${data.firmware.map((f) => `${f.name} v${String(f.versionMajor)}.${String(f.versionMinor)}`).join(", ")}`,
    );
  } catch {
    /* */
  }
  try {
    const host = await device.getHostInfo();
    data.hosts = host.numHosts;
    // Update host slots.
    const hostSection = document.getElementById("host-section");
    if (hostSection) {
      const rows = hostSection.querySelectorAll(".row-value");
      for (let i = 0; i < rows.length; i++) {
        rows[i]!.textContent = i === host.currentHost ? "● active" : "○";
      }
    }
  } catch {
    /* */
  }

  // Full re-render with all data loaded.
  renderDevicePage(data);
}

function updateHeader(): void {
  const meta = document.querySelector(".header .meta");
  if (meta && currentData?.battery) {
    meta.textContent = `${String(currentData.battery.percentage)}%`;
  }
}

// Start.
showConnect();
log("Ready.");
