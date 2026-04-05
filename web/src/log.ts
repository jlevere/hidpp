/// Debug log — shows in-page panel AND streams to local log server.

const LOG_SERVER = "http://localhost:5555/log";

let logEl: HTMLElement | null = null;

function ensurePanel(): HTMLElement {
  if (logEl) return logEl;
  logEl = document.createElement("div");
  logEl.id = "debug-log";
  logEl.style.cssText = `
    position: fixed; bottom: 0; left: 0; right: 0;
    max-height: 200px; overflow-y: auto;
    background: #111; border-top: 1px solid #333;
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 0.75rem; padding: 0.5rem;
    color: #aaa; z-index: 9999;
  `;
  document.body.append(logEl);
  return logEl;
}

function timestamp(): string {
  return new Date().toLocaleTimeString("en-US", { hour12: false });
}

function addLine(msg: string, color?: string): void {
  const panel = ensurePanel();
  const line = document.createElement("div");
  if (color !== undefined && color !== "") line.style.color = color;
  line.textContent = msg;
  panel.append(line);
  panel.scrollTop = panel.scrollHeight;
}

function send(msg: string): void {
  // Fire-and-forget POST to log server.
  fetch(LOG_SERVER, { method: "POST", body: msg }).catch(() => {
    // Log server not running — that's fine.
  });
}

export function log(msg: string): void {
  const line = `[${timestamp()}] ${msg}`;
  addLine(line);
  send(line);
  console.log(`[hidpp] ${msg}`);
}

export function logError(msg: string): void {
  const line = `[${timestamp()}] ERROR: ${msg}`;
  addLine(line, "#f87171");
  send(line);
  console.error(`[hidpp] ${msg}`);
}
