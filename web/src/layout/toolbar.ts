import { el } from "../dom";
import { state, subscribe } from "../state";

export function createToolbar(): HTMLElement {
  const brand = el("span", { class: "toolbar-brand" }, "HID++");
  const deviceName = el("span", { class: "toolbar-device" }, "");
  const battery = el("span", { class: "toolbar-battery" }, "");
  const dot = el("span", { class: "dot" });
  const status = el("div", { class: "toolbar-status" }, battery, dot);

  const toolbar = el("div", { class: "toolbar" }, brand, deviceName, status);

  function render(): void {
    if (state.demo) {
      deviceName.replaceChildren(
        document.createTextNode(state.demoName + " "),
        el("span", { class: "demo-badge" }, "DEMO"),
      );
      battery.textContent = "";
      dot.style.background = "var(--warn)";
    } else if (state.device) {
      deviceName.textContent = state.device.name;
      dot.style.background = "var(--ok)";
      void (async (): Promise<void> => {
        try {
          const bat = await state.device!.getBattery();
          battery.textContent = `${String(bat.percentage)}%`;
        } catch {
          battery.textContent = "";
        }
      })();
    } else {
      deviceName.textContent = "No device";
      battery.textContent = "";
      dot.style.background = "var(--t3)";
    }
  }

  render();
  subscribe(render);

  return toolbar;
}
