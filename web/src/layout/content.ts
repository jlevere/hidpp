import { el } from "../dom";
import { state, subscribe } from "../state";
import { createDpiSection } from "../sections/dpi";
import { createScrollSection } from "../sections/scroll";
import { createButtonsSection } from "../sections/buttons";
import { createHostSection } from "../sections/host";
import { createInfoSection } from "../sections/info";

export function createContent(): HTMLElement {
  const content = el("div", { class: "content" });
  const cache = new Map<string, HTMLElement>();

  function render(): void {
    const dev = state.device;
    const key = `${state.section}-${state.demo ? "demo" : "live"}`;

    if (!dev && !state.demo) {
      content.replaceChildren(el("div", {}, "No device connected."));
      return;
    }

    if (!cache.has(key)) {
      switch (state.section) {
        case "dpi":
          cache.set(key, createDpiSection(dev));
          break;
        case "scroll":
          cache.set(key, createScrollSection(dev));
          break;
        case "buttons":
          cache.set(key, createButtonsSection(dev));
          break;
        case "host":
          cache.set(key, createHostSection(dev));
          break;
        case "info":
          cache.set(key, createInfoSection(dev));
          break;
      }
    }

    const section = cache.get(key);
    if (section) {
      content.replaceChildren(section);
    }
  }

  render();
  subscribe(render);
  return content;
}
