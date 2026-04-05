import type { Device } from "../device";
import { card, row, badge } from "../dom";

export interface BatteryCard {
  root: HTMLElement;
  refresh: (device: Device) => Promise<void>;
}

export function createBatteryCard(): BatteryCard {
  const levelRow = row("Level");
  const statusRow = row("Status");
  const root = card("Battery", levelRow.root, statusRow.root);

  return {
    root,
    async refresh(device: Device) {
      try {
        const bat = await device.getBattery();
        levelRow.value.textContent = `${bat.percentage}%`;

        const variant =
          bat.level === "Critical" || bat.level === "Low"
            ? "warn"
            : "success";
        statusRow.value.replaceChildren(
          badge(bat.level, variant as "warn" | "success"),
          document.createTextNode(` ${bat.charging}`),
        );
      } catch (e) {
        levelRow.value.textContent = `Error: ${e}`;
      }
    },
  };
}
