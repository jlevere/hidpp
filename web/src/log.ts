/// Debug log — hidden by default. Show with ?debug in URL.

const DEBUG = new URLSearchParams(window.location.search).has("debug");

let logEl: HTMLElement | null = null;

function ensurePanel(): HTMLElement | null {
  if (!DEBUG) return null;
  if (logEl) return logEl;
  logEl = document.createElement("div");
  logEl.id = "debug-log";
  document.body.append(logEl);
  return logEl;
}

function timestamp(): string {
  return new Date().toLocaleTimeString("en-US", { hour12: false });
}

export function log(msg: string): void {
  const line = `[${timestamp()}] ${msg}`;
  const panel = ensurePanel();
  if (panel) {
    const div = document.createElement("div");
    div.textContent = line;
    panel.append(div);
    panel.scrollTop = panel.scrollHeight;
  }
  console.log(`[hidpp] ${msg}`);
}

export function logError(msg: string): void {
  const line = `[${timestamp()}] ERROR: ${msg}`;
  const panel = ensurePanel();
  if (panel) {
    const div = document.createElement("div");
    div.style.color = "#c17a7a";
    div.textContent = line;
    panel.append(div);
    panel.scrollTop = panel.scrollHeight;
  }
  console.error(`[hidpp] ${msg}`);
}
