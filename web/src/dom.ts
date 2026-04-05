/// Type-safe DOM helpers. This is the entire "framework."

/** Create an element with attributes and children. */
export function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs?: Record<string, string>,
  ...children: (string | Node)[]
): HTMLElementTagNameMap[K] {
  const elem = document.createElement(tag);
  if (attrs) {
    for (const [k, v] of Object.entries(attrs)) {
      elem.setAttribute(k, v);
    }
  }
  for (const child of children) {
    elem.append(typeof child === "string" ? document.createTextNode(child) : child);
  }
  return elem;
}

/** Query selector — throws if not found. */
export function qs(selector: string, parent: ParentNode = document): HTMLElement {
  const found = parent.querySelector<HTMLElement>(selector);
  if (!found) throw new Error(`Element not found: ${selector}`);
  return found;
}

/** Set text content. */
export function setText(element: HTMLElement, text: string): void {
  element.textContent = text;
}

/** Create a card container. */
export function card(title: string, ...children: Node[]): HTMLDivElement {
  const heading = el("h2", {}, title);
  const div = el("div", { class: "card" }, heading, ...children);
  return div;
}

/** Create a label/value row. */
export function row(
  label: string,
  valueEl?: HTMLElement,
): { root: HTMLDivElement; value: HTMLSpanElement } {
  const valueSpan = valueEl ?? el("span", { class: "value" }, "-");
  const root = el("div", { class: "row" }, el("span", { class: "label" }, label), valueSpan);
  return { root, value: valueSpan };
}

/** Create a badge element. */
export function badge(
  text: string,
  variant: "default" | "warn" | "success" | "info" = "default",
): HTMLSpanElement {
  const cls = variant === "default" ? "badge" : `badge ${variant}`;
  return el("span", { class: cls }, text);
}
