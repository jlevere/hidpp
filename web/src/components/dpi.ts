import type { Device } from "../device";
import { card, row, el } from "../dom";

export interface DpiCard {
  root: HTMLElement;
  refresh: (device: Device) => Promise<void>;
}

export function createDpiCard(): DpiCard {
  const currentRow = row("Current");

  const slider = el("input", {
    type: "range",
    min: "200",
    max: "8000",
    step: "50",
    value: "1000",
  }) as HTMLInputElement;

  const numInput = el("input", {
    type: "number",
    min: "200",
    max: "8000",
    step: "50",
    value: "1000",
  }) as HTMLInputElement;

  const applyBtn = el("button", {}, "Apply");
  const controls = el("div", { class: "controls" }, slider, numInput, applyBtn);

  const root = card("DPI", currentRow.root, controls);

  // Sync slider and input.
  slider.addEventListener("input", () => {
    numInput.value = slider.value;
  });
  numInput.addEventListener("input", () => {
    slider.value = numInput.value;
  });

  let currentDevice: Device | null = null;

  applyBtn.addEventListener("click", async () => {
    if (!currentDevice) return;
    const dpi = parseInt(numInput.value, 10);
    if (isNaN(dpi)) return;

    applyBtn.setAttribute("disabled", "");
    try {
      const applied = await currentDevice.setDpi(dpi);
      currentRow.value.textContent = String(applied);
      slider.value = String(applied);
      numInput.value = String(applied);
    } finally {
      applyBtn.removeAttribute("disabled");
    }
  });

  return {
    root,
    async refresh(device: Device) {
      currentDevice = device;
      try {
        const dpi = await device.getDpi();
        currentRow.value.textContent = String(dpi);
        slider.value = String(dpi);
        numInput.value = String(dpi);
      } catch (e) {
        currentRow.value.textContent = `Error: ${e}`;
      }
    },
  };
}
