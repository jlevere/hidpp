import type { Device } from "../device";
import { card, row } from "../dom";

export interface HostCard {
  root: HTMLElement;
  refresh: (device: Device) => Promise<void>;
}

export function createHostCard(): HostCard {
  const hostRow = row("Current Host");
  const root = card("Easy-Switch", hostRow.root);

  return {
    root,
    async refresh(device: Device) {
      try {
        const info = await device.getHostInfo();
        hostRow.value.textContent = `${info.currentHost + 1} of ${info.numHosts}`;
      } catch (e) {
        hostRow.value.textContent = `Error: ${e}`;
      }
    },
  };
}
