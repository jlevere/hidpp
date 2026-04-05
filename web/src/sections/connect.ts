import { el } from "../dom";

interface ConnectOpts {
  readonly supported: boolean;
  readonly onConnect: () => void;
}

export function createConnectScreen(opts: ConnectOpts): HTMLElement {
  const root = el("div", { class: "connect-screen" });

  root.append(
    el("h1", {}, "HID++ Configurator"),
    el("p", {}, "Configure Logitech devices from your browser. No software to install."),
  );

  if (!opts.supported) {
    root.append(
      el("p", { style: "color: var(--warn)" }, "WebHID not available. Use Chrome or Edge."),
    );
    return root;
  }

  const status = el("span", { class: "status" });
  const btn = el("button", {}, "Connect Device");

  btn.addEventListener("click", () => {
    btn.setAttribute("disabled", "");
    status.textContent = "Connecting...";
    opts.onConnect();
  });

  root.append(btn, status);
  return root;
}

export function resetConnectButton(root: HTMLElement): void {
  const btn = root.querySelector("button");
  const status = root.querySelector(".status");
  if (btn) btn.removeAttribute("disabled");
  if (status) status.textContent = "";
}
