import { connect, isSupported, type Device } from "./device";
import { el } from "./dom";
import { log, logError } from "./log";
import { setDevice, setDemoMode, type SupportedSections } from "./state";
import { createConnectScreen, resetConnectButton } from "./sections/connect";
import { createDemoPicker, type DemoDevice } from "./sections/demo-picker";
import { createToolbar } from "./layout/toolbar";
import { createSidebar } from "./layout/sidebar";
import { createContent } from "./layout/content";
import init, { WasmDevice } from "hidpp-web";

log("HID++ Configurator starting...");

const app = document.getElementById("app") as HTMLDivElement;

let wasmReady = false;

// Connect screen with "Browse devices" link.
const connectScreen = createConnectScreen({
  supported: isSupported(),
  onConnect: (): void => {
    void (async (): Promise<void> => {
      try {
        log("Connecting...");
        const device = await connect();
        log(`Connected: ${device.name}`);
        showApp(device);
      } catch (e) {
        logError(String(e));
        resetConnectButton(connectScreen);
      }
    })();
  },
});

// Add "Browse devices" link below connect button.
const browseLink = el("button", { class: "browse-link" }, "Browse all devices →");
browseLink.addEventListener("click", () => {
  void showDemoPicker();
});
connectScreen.append(browseLink);

app.append(connectScreen);
log("Ready.");

function showApp(device: Device): void {
  const sections = device.getSupportedSections() as SupportedSections;
  setDevice(device, sections);

  app.replaceChildren();
  app.classList.add("app");
  app.append(createToolbar(), createSidebar(), createContent());
}

function showDemoApp(demo: DemoDevice): void {
  const sections: SupportedSections = {
    dpi: demo.hasDpi,
    scroll: demo.hasSmartshift,
    buttons: demo.buttons.length > 0,
    host: demo.hosts > 0,
    battery: demo.hasBattery,
    thumbwheel: false,
    firmware: true,
    friendlyName: false,
    hostsInfo: false,
  };

  setDemoMode(demo.name, sections);

  app.replaceChildren();
  app.classList.add("app");
  app.append(createToolbar(), createSidebar(), createContent());
}

async function showDemoPicker(): Promise<void> {
  if (!wasmReady) {
    await init();
    wasmReady = true;
  }

  const catalogJson = WasmDevice.getDeviceCatalog();
  const picker = createDemoPicker(catalogJson, (device) => {
    showDemoApp(device);
  });

  app.replaceChildren(picker);
}
