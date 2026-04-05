import type { Device } from "../device";
import { card, row, badge, el } from "../dom";

export interface SmartShiftCard {
  root: HTMLElement;
  refresh: (device: Device) => Promise<void>;
}

export function createSmartShiftCard(): SmartShiftCard {
  const modeRow = row("Mode");
  const disengage = row("Auto-disengage");
  const torqueRow = row("Torque");

  const ratchetBtn = el("button", {}, "Ratchet");
  const freeBtn = el("button", {}, "Free Spin");
  const controls = el("div", { class: "controls" }, ratchetBtn, freeBtn);

  const root = card(
    "SmartShift",
    modeRow.root,
    disengage.root,
    torqueRow.root,
    controls,
  );

  let currentDevice: Device | null = null;

  function updateDisplay(mode: string, ad: number, torque: number): void {
    const variant = mode === "Ratchet" ? "default" : "success";
    modeRow.value.replaceChildren(badge(mode, variant as "default" | "success"));
    disengage.value.textContent = String(ad);
    torqueRow.value.textContent = String(torque);
  }

  async function setMode(mode: string): Promise<void> {
    if (!currentDevice) return;
    ratchetBtn.setAttribute("disabled", "");
    freeBtn.setAttribute("disabled", "");
    try {
      const result = await currentDevice.setSmartShift(mode, 0, 0);
      updateDisplay(result.mode, result.autoDisengage, result.torque);
    } finally {
      ratchetBtn.removeAttribute("disabled");
      freeBtn.removeAttribute("disabled");
    }
  }

  ratchetBtn.addEventListener("click", () => void setMode("ratchet"));
  freeBtn.addEventListener("click", () => void setMode("freespin"));

  return {
    root,
    async refresh(device: Device) {
      currentDevice = device;
      try {
        const state = await device.getSmartShift();
        updateDisplay(state.mode, state.autoDisengage, state.torque);
      } catch (e) {
        modeRow.value.textContent = `Error: ${e}`;
      }
    },
  };
}
