/// Debug log — hidden by default. Show with ?debug in URL.
/// Also streams to localhost:5555 if a log server is running.

const LOG_SERVER = "http://localhost:5555/log";
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

function send(msg: string): void {
  if (!DEBUG) return;
  fetch(LOG_SERVER, { method: "POST", body: msg }).catch(() => {});
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
  send(line);
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
  send(line);
  console.error(`[hidpp] ${msg}`);
}
