import { el } from "../dom";
import { log } from "../log";

interface CatalogDevice {
  displayName: string;
  modelId: string;
  type: string;
  capabilities: {
    hasHighResolutionSensor?: boolean;
    highResolutionSensorInfo?: { defaultDpiValueSensorOff: number };
    flow?: { hostCount: number };
    specialKeys?: { programmable: number[] };
    hasBatteryStatus?: boolean;
    unified_battery?: boolean;
    scroll_wheel_capabilities?: { smartshift: boolean };
    fnInversion?: boolean;
  };
}

export interface DemoDevice {
  name: string;
  modelId: string;
  type: string;
  dpiDefault: number;
  buttons: number[];
  hosts: number;
  hasDpi: boolean;
  hasSmartshift: boolean;
  hasBattery: boolean;
}

export function createDemoPicker(
  catalogJson: string,
  onSelect: (device: DemoDevice) => void,
): HTMLElement {
  const root = el("div", { class: "demo-picker" });
  root.append(
    el("h1", {}, "Browse Devices"),
    el(
      "p",
      { style: "color: var(--t2); margin-bottom: 20px" },
      "Explore settings for any Logitech HID++ device.",
    ),
  );

  const catalog: { devices: CatalogDevice[] } = JSON.parse(catalogJson) as {
    devices: CatalogDevice[];
  };

  // Group by type, filter out receivers/cameras/virtual.
  const groups = new Map<string, CatalogDevice[]>();
  const typeOrder = [
    "MOUSE",
    "KEYBOARD",
    "TRACKBALL",
    "PRESENTER",
    "TOUCHPAD",
    "DIAL",
    "CONTEXTUAL_KEYS",
    "ILLUMINATION_LIGHT",
  ];

  for (const dev of catalog.devices) {
    if (["RECEIVER", "VIRTUAL_DEVICE", "CAMERA"].includes(dev.type)) continue;
    const existing = groups.get(dev.type) ?? [];
    existing.push(dev);
    groups.set(dev.type, existing);
  }

  const TYPE_LABELS: Record<string, string> = {
    MOUSE: "Mice",
    KEYBOARD: "Keyboards",
    TRACKBALL: "Trackballs",
    PRESENTER: "Presenters",
    TOUCHPAD: "Touchpads",
    DIAL: "Dials",
    CONTEXTUAL_KEYS: "Keypads",
    ILLUMINATION_LIGHT: "Lights",
  };

  for (const type of typeOrder) {
    const devices = groups.get(type);
    if (!devices || devices.length === 0) continue;

    root.append(el("div", { class: "sidebar-label" }, TYPE_LABELS[type] ?? type));

    const grid = el("div", { class: "demo-grid" });
    for (const dev of devices) {
      const card = el("button", { class: "demo-card" }, dev.displayName);
      card.addEventListener("click", () => {
        log(`Demo: selected ${dev.displayName}`);
        const caps = dev.capabilities;
        onSelect({
          name: dev.displayName,
          modelId: dev.modelId,
          type: dev.type,
          dpiDefault: caps.highResolutionSensorInfo?.defaultDpiValueSensorOff ?? 1000,
          buttons: caps.specialKeys?.programmable ?? [],
          hosts: caps.flow?.hostCount ?? 0,
          hasDpi: caps.hasHighResolutionSensor ?? false,
          hasSmartshift: caps.scroll_wheel_capabilities?.smartshift ?? false,
          hasBattery: (caps.hasBatteryStatus ?? false) || (caps.unified_battery ?? false),
        });
      });
      grid.append(card);
    }
    root.append(grid);
  }

  return root;
}
