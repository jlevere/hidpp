import type { Device } from "../device";
import { el } from "../dom";
import { logError } from "../log";

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

export function createButtonsSection(device: Device): HTMLElement {
  const root = el("div", {});
  root.append(el("div", { class: "section-title" }, "Buttons"));

  // Device image.
  const imgWrap = el("div", { class: "device-hero" });
  const imgInner = el("div", { class: "device-img-wrap" });
  const img = el("img", {
    src: "/logi-re/devices/assets/mx-master-3s/side.png",
    alt: "Device",
    class: "device-img",
    draggable: "false",
  });
  imgInner.append(img);
  imgWrap.append(imgInner);
  root.append(imgWrap);

  // Button list card.
  const card = el("div", { class: "card" });
  card.append(el("div", { class: "card-title" }, "Remappable Controls"));
  const listEl = el("div", {});
  card.append(listEl);
  root.append(card);

  // Load buttons.
  void (async (): Promise<void> => {
    try {
      const buttons = await device.getButtons();
      for (const btn of buttons) {
        const b = btn as Record<string, number | boolean>;
        const cid = b.cid as number;
        const name = BUTTON_NAMES[cid] ?? `Control ${String(cid)}`;
        const row = el(
          "div",
          { class: "btn-row" },
          el("span", { class: "btn-row-cid" }, `0x${cid.toString(16).padStart(4, "0")}`),
          el("span", { class: "btn-row-name" }, name),
          b.divertable === true
            ? el("span", { class: "badge badge-accent" }, "divertable")
            : el("span", {}),
        );
        listEl.append(row);
      }
    } catch (e) {
      listEl.textContent = `Error: ${String(e)}`;
      logError(`Buttons: ${String(e)}`);
    }
  })();

  return root;
}
