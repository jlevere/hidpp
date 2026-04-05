import type { Device } from "../device";
import { el } from "../dom";
import { log, logError } from "../log";
import { state } from "../state";

export function createInfoSection(device: Device): HTMLElement {
  const root = el("div", {});
  root.append(el("div", { class: "section-title" }, "Device Info"));

  // Device info card.
  const infoCard = el("div", { class: "card" });
  infoCard.append(
    el("div", { class: "card-title" }, "Device"),
    el(
      "div",
      { class: "row" },
      el("span", { class: "row-label" }, "Name"),
      el("span", { class: "row-value" }, device.name),
    ),
    el(
      "div",
      { class: "row" },
      el("span", { class: "row-label" }, "Features"),
      el("span", { class: "row-value" }, String(device.featureCount)),
    ),
  );
  root.append(infoCard);

  // Firmware card.
  if (state.sections.firmware) {
    const fwCard = el("div", { class: "card" });
    fwCard.append(el("div", { class: "card-title" }, "Firmware"));
    const fwList = el("div", {});
    fwCard.append(fwList);
    root.append(fwCard);

    void (async (): Promise<void> => {
      try {
        const fw = await device.getFirmware();
        for (const ent of fw) {
          const e = ent as Record<string, string | number>;
          fwList.append(
            el(
              "div",
              { class: "fw-row" },
              el("span", { class: "fw-name" }, String(e.name)),
              el("span", { class: "fw-type" }, String(e.type)),
              el(
                "span",
                { class: "fw-ver" },
                `v${(e.versionMajor as number).toString(16)}.${(e.versionMinor as number).toString(16).padStart(2, "0")} build ${String(e.build)}`,
              ),
            ),
          );
        }
      } catch (e) {
        fwList.textContent = `Error: ${String(e)}`;
      }
    })();
  }

  // Config export/import card.
  const configCard = el("div", { class: "card" });
  configCard.append(el("div", { class: "card-title" }, "Configuration"));

  const exportBtn = el("button", { class: "btn-ghost" }, "Export TOML");
  const importBtn = el("button", { class: "btn-ghost" }, "Import TOML");
  const configControls = el("div", { class: "controls" }, exportBtn, importBtn);
  configCard.append(configControls);
  root.append(configCard);

  exportBtn.addEventListener("click", () => {
    void (async (): Promise<void> => {
      try {
        const toml = await device.exportConfig();
        log(`Config exported (${String(toml.length)} chars)`);
        // Copy to clipboard.
        await navigator.clipboard.writeText(toml);
        exportBtn.textContent = "Copied!";
        setTimeout(() => {
          exportBtn.textContent = "Export TOML";
        }, 2000);
      } catch (e) {
        logError(`Export: ${String(e)}`);
      }
    })();
  });

  // Feature list card.
  const featCard = el("div", { class: "card" });
  featCard.append(el("div", { class: "card-title" }, "HID++ Features"));
  const featList = el("div", {});
  featCard.append(featList);
  root.append(featCard);

  const features = device.getFeatures();
  for (const f of features) {
    featList.append(
      el(
        "div",
        { class: "feat-row" },
        el("span", { class: "feat-id" }, f.id),
        el("span", { class: "feat-name" }, f.name),
      ),
    );
  }

  return root;
}
