import { el } from "../dom";

interface ConnectScreenOpts {
  supported: boolean;
  onConnect: () => void;
}

export function createConnectScreen(opts: ConnectScreenOpts): HTMLElement {
  const root = el("div", { class: "connect-screen" });

  const title = el("h1", {}, "HID++ Configurator");
  const subtitle = el(
    "p",
    { class: "subtitle" },
    "Configure Logitech HID++ 2.0 devices from your browser. No software to install.",
  );

  root.append(title, subtitle);

  if (!opts.supported) {
    const notice = el(
      "p",
      { class: "notice" },
      "WebHID is not available. Please use Chrome or Edge.",
    );
    root.append(notice);
    return root;
  }

  const status = el("span", { class: "status" });
  const btn = el("button", { class: "btn-primary" }, "Connect Device");

  btn.addEventListener("click", () => {
    btn.setAttribute("disabled", "");
    status.textContent = "Connecting...";
    opts.onConnect();
  });

  root.append(el("div", { class: "connect-row" }, btn, status));

  return root;
}
