import type { Device } from "../device";
import { el } from "../dom";
import { logError } from "../log";
import { state } from "../state";

export function createHostSection(device: Device): HTMLElement {
  const root = el("div", {});
  root.append(el("div", { class: "section-title" }, "Easy-Switch"));

  const card = el("div", { class: "card" });
  card.append(el("div", { class: "card-title" }, "Host Slots"));
  const slotsEl = el("div", {});
  card.append(slotsEl);
  root.append(card);

  void (async (): Promise<void> => {
    try {
      const info = await device.getHostInfo();

      for (let i = 0; i < info.numHosts; i++) {
        const isCurrent = i === info.currentHost;
        const marker = isCurrent ? "●" : "○";
        const slotLabel = `${marker}  Slot ${String(i + 1)}`;

        let osInfo = "";
        if (state.sections.hostsInfo) {
          try {
            const os = await device.getHostOsVersion(i);
            osInfo = ` — ${os.osType}`;
          } catch {
            /* no OS info for this slot */
          }
        }

        const row = el(
          "div",
          { class: "row" },
          el("span", { class: "row-label" }, slotLabel + osInfo),
          isCurrent
            ? el("span", { class: "badge badge-ok" }, "active")
            : el("span", { class: "row-value", style: "color: var(--t3)" }, "—"),
        );
        slotsEl.append(row);
      }
    } catch (e) {
      slotsEl.textContent = `Error: ${String(e)}`;
      logError(`Host: ${String(e)}`);
    }
  })();

  // Friendly name card.
  if (state.sections.friendlyName) {
    const nameCard = el("div", { class: "card" });
    nameCard.append(el("div", { class: "card-title" }, "Bluetooth Name"));
    const nameVal = el("span", { class: "row-value" }, "—");
    nameCard.append(
      el("div", { class: "row" }, el("span", { class: "row-label" }, "Name"), nameVal),
    );
    root.append(nameCard);

    void (async (): Promise<void> => {
      try {
        const name = await device.getFriendlyName();
        nameVal.textContent = name;
      } catch {
        /* no friendly name */
      }
    })();
  }

  return root;
}
