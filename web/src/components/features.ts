import type { Feature } from "../types";
import { card, el } from "../dom";

export function createFeaturesCard(features: Feature[]): { root: HTMLElement } {
  const list = el("div", { class: "feature-list" });

  for (const f of features) {
    const line = el(
      "div",
      { class: "feature-row" },
      el("code", {}, f.id),
      document.createTextNode(` ${f.name}`),
    );
    list.append(line);
  }

  const toggle = el("button", { class: "btn-text" }, "Show all features");
  list.style.display = "none";

  toggle.addEventListener("click", () => {
    const visible = list.style.display !== "none";
    list.style.display = visible ? "none" : "";
    toggle.textContent = visible ? "Show all features" : "Hide features";
  });

  const root = card("Features", toggle, list);
  return { root };
}
