import { connect, isSupported, type Device } from "./device";
import { log, logError } from "./log";
import { setDevice, type SupportedSections } from "./state";
import { createConnectScreen, resetConnectButton } from "./sections/connect";
import { createToolbar } from "./layout/toolbar";
import { createSidebar } from "./layout/sidebar";
import { createContent } from "./layout/content";

log("HID++ Configurator starting...");

const app = document.getElementById("app") as HTMLDivElement;

// Show connect screen first.
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

app.append(connectScreen);
log("Ready.");

function showApp(device: Device): void {
  // Query which sections this device supports.
  const sections = device.getSupportedSections() as SupportedSections;
  setDevice(device, sections);

  // Build the app layout.
  app.replaceChildren();
  app.classList.add("app");

  const toolbar = createToolbar();
  const sidebar = createSidebar();
  const content = createContent();

  app.append(toolbar, sidebar, content);
}
