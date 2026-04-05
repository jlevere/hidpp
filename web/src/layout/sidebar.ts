import { el } from "../dom";
import { state, setSection, subscribe, type Section } from "../state";

interface NavItem {
  readonly id: Section;
  readonly label: string;
  readonly condition: () => boolean;
}

const NAV_ITEMS: readonly NavItem[] = [
  { id: "dpi", label: "DPI", condition: () => state.sections.dpi },
  { id: "scroll", label: "Scroll", condition: () => state.sections.scroll },
  { id: "buttons", label: "Buttons", condition: () => state.sections.buttons },
  { id: "host", label: "Host", condition: () => state.sections.host },
  { id: "info", label: "Info", condition: () => true },
];

export function createSidebar(): HTMLElement {
  const sidebar = el("nav", { class: "sidebar" });
  const label = el("div", { class: "sidebar-label" }, "Configure");
  sidebar.append(label);

  const items = new Map<Section, HTMLButtonElement>();

  for (const nav of NAV_ITEMS) {
    const btn = el("button", { class: "sidebar-item" }, nav.label);
    btn.addEventListener("click", () => {
      setSection(nav.id);
    });
    items.set(nav.id, btn);
    sidebar.append(btn);
  }

  function render(): void {
    for (const nav of NAV_ITEMS) {
      const btn = items.get(nav.id);
      if (!btn) continue;
      btn.style.display = nav.condition() ? "" : "none";
      btn.classList.toggle("active", state.section === nav.id);
    }
  }

  render();
  subscribe(render);

  return sidebar;
}
