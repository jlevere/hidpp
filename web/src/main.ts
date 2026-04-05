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
  root.append(el("h1", {}, "HID++"), el("p", {}, "Configure Logitech devices from your browser."));

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
    smartShiftAutoDisengage: 10,
    hiresWheel: null,
    thumbwheel: null,
    buttons: (
      (caps?.specialKeys as { programmable?: number[] } | undefined)?.programmable ?? []
    ).map((cid) => ({ cid, divertable: true, diverted: false })),
    hosts: (caps?.flow as { hostCount?: number } | undefined)?.hostCount ?? 0,
    hostCurrent: 0,
    hostSlots: [],
    firmware: [],
    features: [],
    friendlyName: null,
    device: null,
  });
}

// ── Live device view ──

function showDevice(device: Device): void {
  // Show loading state briefly, then load ALL data before rendering.
  app.replaceChildren(el("div", { class: "connect" }, el("p", {}, "Loading...")));

  void (async (): Promise<void> => {
    const data: PageData = {
      name: device.name,
      demo: false,
      battery: null,
      dpi: null,
      dpiRange: null,
      smartShift: null,
      smartShiftAutoDisengage: 0,
      hiresWheel: null,
      thumbwheel: null,
      buttons: [],
      hosts: 0,
      hostCurrent: 0,
      hostSlots: [],
      firmware: [],
      features: device.getFeatures() as { id: string; name: string }[],
      friendlyName: null,
      device,
    };

    // Load all data in parallel.
    const results = await Promise.allSettled([
      device.getBattery().then((b) => {
        data.battery = b;
      }),
      device.getDpi().then((d) => {
        data.dpi = d;
      }),
      device.getSmartShift().then((s) => {
        data.smartShift = s.mode;
        data.smartShiftAutoDisengage = s.autoDisengage;
      }),
      device
        .getHiResWheel()
        .then((hw) => {
          data.hiresWheel = hw;
        })
        .catch(() => {
          /* optional */
        }),
      device
        .getThumbwheel()
        .then((tw) => {
          data.thumbwheel = tw;
        })
        .catch(() => {
          /* optional */
        }),
      device.getButtons().then((btns) => {
        data.buttons = (btns as { cid: number; divertable: boolean }[]).map((b) => ({
          cid: b.cid,
          divertable: b.divertable,
          diverted: false,
        }));
      }),
      device.getHostInfo().then(async (h) => {
        data.hosts = h.numHosts;
        data.hostCurrent = h.currentHost;
        // Load per-slot OS info.
        for (let i = 0; i < h.numHosts; i++) {
          try {
            const os = await device.getHostOsVersion(i);
            data.hostSlots.push(os);
          } catch {
            data.hostSlots.push({ osType: "Unknown", major: 0, minor: 0 });
          }
        }
      }),
      device.getFirmware().then((fw) => {
        data.firmware = fw as typeof data.firmware;
      }),
      device
        .getFriendlyName()
        .then((n) => {
          data.friendlyName = n;
        })
        .catch(() => {
          /* optional */
        }),
    ]);

    for (const r of results) {
      if (r.status === "rejected") {
        log(`Load warning: ${String(r.reason)}`);
      }
    }

    renderDevicePage(data);

    // Lazy-load button divert status after page is visible.
    if (data.buttons.length > 0) {
      void loadButtonReporting(device, data);
    }
  })();
}

async function loadButtonReporting(device: Device, data: PageData): Promise<void> {
  for (const btn of data.buttons) {
    if (!btn.divertable) continue;
    try {
      const r = await device.getButtonReporting(btn.cid);
      btn.diverted = r.diverted;
      // Update the button's toggle in the DOM.
      const toggle = document
        .querySelector(`.button-item .cid[data-cid="${String(btn.cid)}"]`)
        ?.parentElement?.querySelector("button");
      if (toggle) {
        toggle.textContent = r.diverted ? "diverted" : "default";
        toggle.classList.toggle("active", r.diverted);
      }
    } catch {
      /* */
    }
  }
}

interface PageData {
  name: string;
  demo: boolean;
  battery: { percentage: number; level: string; charging: string } | null;
  dpi: number | null;
  dpiRange: { min: number; max: number; step: number } | null;
  smartShift: string | null;
  smartShiftAutoDisengage: number;
  hiresWheel: { highResolution: boolean; inverted: boolean } | null;
  thumbwheel: { mode: string; inverted: boolean } | null;
  buttons: { cid: number; divertable: boolean; diverted: boolean }[];
  hosts: number;
  hostCurrent: number;
  hostSlots: { osType: string; major: number; minor: number }[];
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
  const metaParts: string[] = [];
  if (data.battery) metaParts.push(`${String(data.battery.percentage)}%`);
  if (data.friendlyName) metaParts.push(data.friendlyName);
  const meta = el("span", { class: "meta" }, metaParts.join(" · "));
  if (data.demo) {
    meta.textContent = "";
    meta.append(el("span", { class: "demo-tag" }, "DEMO"));
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
      // Scroll to and highlight the button in the list.
      const row = document.querySelector(
        `.button-item .cid[data-cid="${String(h.cid)}"]`,
      )?.parentElement;
      if (row) {
        row.scrollIntoView({ behavior: "smooth", block: "center" });
        row.style.background = "var(--dim)";
        setTimeout(() => {
          row.style.background = "";
        }, 1500);
      }
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

    // HiResWheel.
    if (data.hiresWheel) {
      const hw = data.hiresWheel;
      const hiresBtn = el("button", {}, hw.highResolution ? "on" : "off");
      hiresBtn.classList.toggle("active", hw.highResolution);
      if (!data.demo) {
        hiresBtn.addEventListener("click", () => {
          void (async (): Promise<void> => {
            if (!data.device) return;
            try {
              const result = await data.device.setHiResWheel(!hw.highResolution, hw.inverted);
              hiresBtn.textContent = result.highResolution ? "on" : "off";
              hiresBtn.classList.toggle("active", result.highResolution);
              hw.highResolution = result.highResolution;
            } catch (e) {
              logError(`HiRes: ${String(e)}`);
            }
          })();
        });
      } else {
        hiresBtn.setAttribute("disabled", "");
      }
      section.append(
        el(
          "div",
          { class: "row" },
          el("span", { class: "row-label" }, "High resolution"),
          hiresBtn,
        ),
      );
    }

    // Thumbwheel.
    if (data.thumbwheel) {
      section.append(
        el(
          "div",
          { class: "row" },
          el("span", { class: "row-label" }, "Thumbwheel"),
          el("span", { class: "row-value" }, data.thumbwheel.mode),
        ),
      );
    }

    root.append(section);
  }

  // Buttons section.
  if (data.buttons.length > 0) {
    const section = el("div", { class: "section" });
    section.append(el("div", { class: "section-label" }, "Buttons"));
    for (const btn of data.buttons) {
      const name = BUTTON_NAMES[btn.cid] ?? `Control ${String(btn.cid)}`;
      const statusLabel = btn.diverted ? "software" : "standard";
      const item = el(
        "div",
        { class: "button-item" },
        el(
          "span",
          { class: "cid", "data-cid": String(btn.cid) },
          `0x${btn.cid.toString(16).padStart(4, "0")}`,
        ),
        el("span", { class: "name" }, name),
      );
      if (btn.divertable) {
        const tag = el("span", { class: "tag" }, statusLabel);
        tag.classList.toggle("active", btn.diverted);
        item.append(tag);
      }
      section.append(item);
    }
    root.append(section);
  }

  // Host section.
  if (data.hosts > 0) {
    const section = el("div", { class: "section", id: "host-section" });
    section.append(el("div", { class: "section-label" }, "Easy-Switch"));
    for (let i = 0; i < data.hosts; i++) {
      const active = i === data.hostCurrent;
      const slot = data.hostSlots[i];
      const osLabel = slot && slot.osType !== "Unknown" ? ` · ${slot.osType}` : "";
      const right = el("span", {});
      if (active) {
        right.append(el("span", { style: "color: var(--success); font-size: 0.75rem" }, "active"));
      } else if (!data.demo && data.device) {
        const switchBtn = el(
          "button",
          { style: "font-size: 0.7rem; padding: 0.2rem 0.6rem" },
          "switch",
        );
        const hostIdx = i;
        switchBtn.addEventListener("click", () => {
          if (confirm(`Switch to Slot ${String(hostIdx + 1)}? This will disconnect the mouse.`)) {
            void (async (): Promise<void> => {
              try {
                await data.device!.switchHost(hostIdx);
              } catch (e) {
                logError(`Switch host: ${String(e)}`);
              }
            })();
          }
        });
        right.append(switchBtn);
      }
      section.append(
        el(
          "div",
          { class: "row" },
          el(
            "span",
            { class: "row-label" },
            `${active ? "●" : "○"} Slot ${String(i + 1)}${osLabel}`,
          ),
          right,
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
    const known = data.features.filter(
      (f) => f.name !== "Unknown" && !f.name.startsWith("Unknown"),
    );
    const unknown = data.features.filter(
      (f) => f.name === "Unknown" || f.name.startsWith("Unknown"),
    );

    const section = el("div", { class: "section" });
    section.append(
      el("div", { class: "section-label" }, `Features · ${String(data.features.length)}`),
    );
    const wrap = el("div", {});
    for (const f of known) {
      wrap.append(
        el(
          "span",
          { class: "feature-item" },
          el("span", { class: "fid" }, f.id),
          document.createTextNode(` ${f.name}`),
        ),
      );
    }

    if (unknown.length > 0) {
      const moreWrap = el("span", { class: "feature-item" });
      const moreBtn = el("button", { class: "more-toggle" }, `+${String(unknown.length)} unknown`);
      const moreList = el("span", { style: "display: none" });
      for (const f of unknown) {
        moreList.append(
          el(
            "span",
            { class: "feature-item" },
            el("span", { class: "fid" }, f.id),
            document.createTextNode(` ${f.name}`),
          ),
        );
      }
      moreBtn.addEventListener("click", () => {
        const showing = moreList.style.display !== "none";
        moreList.style.display = showing ? "none" : "";
        moreBtn.textContent = showing ? `+${String(unknown.length)} unknown` : "hide";
      });
      moreWrap.append(moreBtn, moreList);
      wrap.append(moreWrap);
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
    const ad = currentData.smartShiftAutoDisengage;
    const result = await currentData.device.setSmartShift(mode, ad, 0);
    if (smartShiftLabel) smartShiftLabel.textContent = result.mode;
    ratchetBtn?.classList.toggle("active", result.mode === "Ratchet");
    freeBtn?.classList.toggle("active", result.mode === "FreeScroll");
    log(`SmartShift → ${result.mode}`);
  } catch (e) {
    logError(`SmartShift: ${String(e)}`);
  }
}

// loadLiveData and updateHeader removed — data loaded upfront in showDevice().

// Start.
showConnect();
log("Ready.");
