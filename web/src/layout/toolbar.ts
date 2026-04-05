import { el } from "../dom";
import { state } from "../state";

export function createToolbar(): HTMLElement {
  const brand = el("span", { class: "toolbar-brand" }, "HID++");
  const deviceName = el("span", { class: "toolbar-device" }, "");
  const battery = el("span", { class: "toolbar-battery" }, "");
  const dot = el("span", { class: "dot" });
  const status = el("div", { class: "toolbar-status" }, battery, dot);

  const toolbar = el("div", { class: "toolbar" }, brand, deviceName, status);

  // Update on state change.
  void (async (): Promise<void> => {
    const update = async (): Promise<void> => {
      const dev = state.device;
      if (!dev) {
        deviceName.textContent = "No device";
        battery.textContent = "";
        dot.style.background = "var(--t3)";
        return;
      }
      deviceName.textContent = dev.name;
      dot.style.background = "var(--ok)";
      try {
        const bat = await dev.getBattery();
        battery.textContent = `${String(bat.percentage)}%`;
      } catch {
        battery.textContent = "";
      }
    };
    // Initial + subscribe.
    await update();
    const { subscribe } = await import("../state");
    subscribe(() => {
      void update();
    });
  })();

  return toolbar;
}
