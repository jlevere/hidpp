import type { Device } from "../device";
import { el } from "../dom";
import { log, logError } from "../log";

const PRESETS = [400, 800, 1200, 1600, 2400, 3200] as const;

export function createDpiSection(device: Device | null): HTMLElement {
  const root = el("div", {});
  const title = el("div", { class: "section-title" }, "DPI");
  const currentVal = el("span", { class: "stat-val" }, "—");
  const stat = el(
    "div",
    { class: "stat" },
    currentVal,
    el("div", { class: "stat-lbl" }, "Current"),
  );

  const slider = el("input", { type: "range", min: "200", max: "8000", step: "50", value: "1000" });
  const numInput = el("input", {
    type: "number",
    min: "200",
    max: "8000",
    step: "50",
    value: "1000",
  });
  const applyBtn = el("button", {}, "Apply");
  const controls = el("div", { class: "controls" }, slider, numInput, applyBtn);

  // Presets.
  const presetRow = el("div", { class: "presets" });
  const presetBtns: HTMLButtonElement[] = [];
  for (const dpi of PRESETS) {
    const btn = el("button", {}, String(dpi));
    btn.addEventListener("click", () => {
      void applyDpi(dpi);
    });
    presetBtns.push(btn);
    presetRow.append(btn);
  }

  const card = el("div", { class: "card" });
  card.append(el("div", { class: "card-title" }, "Pointer Speed"), stat, controls, presetRow);

  root.append(title, card);

  // Sync slider ↔ input.
  slider.addEventListener("input", () => {
    numInput.value = slider.value;
  });
  numInput.addEventListener("input", () => {
    slider.value = numInput.value;
  });

  async function applyDpi(dpi: number): Promise<void> {
    if (!device) return;
    applyBtn.setAttribute("disabled", "");
    try {
      const applied = await device.setDpi(dpi);
      log(`DPI set to ${String(applied)}`);
      updateDisplay(applied);
    } catch (e) {
      logError(`DPI set failed: ${String(e)}`);
    } finally {
      applyBtn.removeAttribute("disabled");
    }
  }

  function updateDisplay(dpi: number): void {
    currentVal.textContent = String(dpi);
    slider.value = String(dpi);
    numInput.value = String(dpi);
    for (const btn of presetBtns) {
      btn.classList.toggle("active", btn.textContent === String(dpi));
    }
  }

  applyBtn.addEventListener("click", () => {
    const dpi = parseInt(numInput.value, 10);
    if (!isNaN(dpi)) void applyDpi(dpi);
  });

  // Initial read.
  if (device) {
    void (async (): Promise<void> => {
      try {
        const dpi = await device.getDpi();
        updateDisplay(dpi);
      } catch (e) {
        currentVal.textContent = "?";
        logError(`DPI read: ${String(e)}`);
      }
    })();
  } else {
    // Demo mode — show default.
    updateDisplay(1000);
    applyBtn.textContent = "Connect to apply";
    applyBtn.setAttribute("disabled", "");
  }

  return root;
}
