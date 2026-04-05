import { connect, isSupported, type Device } from "./device";
import { qs } from "./dom";
import { log, logError } from "./log";
import { createConnectScreen } from "./components/connect";
import { createDeviceInfoCard } from "./components/device-info";
import { createBatteryCard } from "./components/battery";
import { createDpiCard } from "./components/dpi";
import { createSmartShiftCard } from "./components/smart-shift";
import { createHostCard } from "./components/host";
import { createFeaturesCard } from "./components/features";

log("HID++ Configurator starting...");

const app = qs<HTMLDivElement>("#app");

const connectScreen = createConnectScreen({
  supported: isSupported(),
  onConnect: async () => {
    try {
      log("User clicked Connect...");
      const device = await connect();
      log("Device connected, building UI...");
      showDeviceUI(device);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      logError(msg);
      const status = app.querySelector(".status");
      if (status) status.textContent = `Error: ${msg}`;
      const btn = app.querySelector("button");
      if (btn) btn.removeAttribute("disabled");
    }
  },
});

app.append(connectScreen);
log("Ready. Click Connect to begin.");

function showDeviceUI(device: Device): void {
  app.replaceChildren();

  const info = createDeviceInfoCard(device);
  const battery = createBatteryCard();
  const dpi = createDpiCard();
  const smartShift = createSmartShiftCard();
  const host = createHostCard();
  const features = createFeaturesCard(device.getFeatures());

  app.append(
    info,
    battery.root,
    dpi.root,
    smartShift.root,
    host.root,
    features.root,
  );

  log("Reading device settings...");

  void battery.refresh(device).catch((e) => logError(`battery: ${e}`));
  void dpi.refresh(device).catch((e) => logError(`dpi: ${e}`));
  void smartShift.refresh(device).catch((e) => logError(`smartshift: ${e}`));
  void host.refresh(device).catch((e) => logError(`host: ${e}`));

  log("UI ready.");
}
