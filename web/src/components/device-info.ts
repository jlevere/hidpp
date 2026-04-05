import type { Device } from "../device";
import { card, row } from "../dom";

export function createDeviceInfoCard(device: Device): HTMLElement {
  const nameRow = row("Name");
  const featuresRow = row("Features");

  nameRow.value.textContent = device.name;
  featuresRow.value.textContent = String(device.featureCount);

  return card("Device", nameRow.root, featuresRow.root);
}
