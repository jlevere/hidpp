import type { Device } from "../device";
import { el } from "../dom";
import { log, logError } from "../log";
import { state } from "../state";

export function createScrollSection(device: Device | null): HTMLElement {
  const root = el("div", {});
  root.append(el("div", { class: "section-title" }, "Scroll"));

  // SmartShift card.
  const modeLabel = el("span", { class: "row-value" }, "—");
  const ratchetBtn = el("button", {}, "Ratchet");
  const freeBtn = el("button", {}, "Free Spin");
  const toggle = el("div", { class: "toggle" }, ratchetBtn, freeBtn);

  const ssCard = el("div", { class: "card" });
  ssCard.append(
    el("div", { class: "card-title" }, "SmartShift"),
    el("div", { class: "row" }, el("span", { class: "row-label" }, "Mode"), modeLabel),
    toggle,
  );
  root.append(ssCard);

  async function setMode(mode: string): Promise<void> {
    if (!device) return;
    ratchetBtn.setAttribute("disabled", "");
    freeBtn.setAttribute("disabled", "");
    try {
      const result = await device.setSmartShift(mode, 0, 0);
      updateSmartShift(result.mode);
      log(`SmartShift: ${result.mode}`);
    } catch (e) {
      logError(`SmartShift set: ${String(e)}`);
    } finally {
      ratchetBtn.removeAttribute("disabled");
      freeBtn.removeAttribute("disabled");
    }
  }

  function updateSmartShift(mode: string): void {
    modeLabel.textContent = mode;
    ratchetBtn.classList.toggle("active", mode === "Ratchet");
    freeBtn.classList.toggle("active", mode === "FreeScroll");
  }

  ratchetBtn.addEventListener("click", () => {
    void setMode("ratchet");
  });
  freeBtn.addEventListener("click", () => {
    void setMode("freespin");
  });

  // HiResWheel card (if supported).
  if (state.sections.scroll && device) {
    const hiresLabel = el("span", { class: "row-value" }, "—");
    const invertLabel = el("span", { class: "row-value" }, "—");

    const hiresCard = el("div", { class: "card" });
    hiresCard.append(
      el("div", { class: "card-title" }, "Scroll Wheel"),
      el(
        "div",
        { class: "row" },
        el("span", { class: "row-label" }, "High Resolution"),
        hiresLabel,
      ),
      el("div", { class: "row" }, el("span", { class: "row-label" }, "Inverted"), invertLabel),
    );
    root.append(hiresCard);

    void (async (): Promise<void> => {
      try {
        const mode = await device.getHiResWheel();
        hiresLabel.textContent = mode.highResolution ? "On" : "Off";
        invertLabel.textContent = mode.inverted ? "Yes" : "No";
      } catch {
        /* feature not available */
      }
    })();
  }

  // Thumbwheel card (if supported).
  if (state.sections.thumbwheel && device) {
    const twMode = el("span", { class: "row-value" }, "—");
    const twInvert = el("span", { class: "row-value" }, "—");

    const twCard = el("div", { class: "card" });
    twCard.append(
      el("div", { class: "card-title" }, "Thumbwheel"),
      el("div", { class: "row" }, el("span", { class: "row-label" }, "Mode"), twMode),
      el("div", { class: "row" }, el("span", { class: "row-label" }, "Inverted"), twInvert),
    );
    root.append(twCard);

    void (async (): Promise<void> => {
      try {
        const tw = await device.getThumbwheel();
        twMode.textContent = tw.mode;
        twInvert.textContent = tw.inverted ? "Yes" : "No";
      } catch {
        /* feature not available */
      }
    })();
  }

  // Initial SmartShift read.
  if (device) {
    void (async (): Promise<void> => {
      try {
        const ss = await device.getSmartShift();
        updateSmartShift(ss.mode);
      } catch (e) {
        modeLabel.textContent = "?";
        logError(`SmartShift read: ${String(e)}`);
      }
    })();
  } else {
    // Demo mode.
    updateSmartShift("Ratchet");
    ratchetBtn.setAttribute("disabled", "");
    freeBtn.setAttribute("disabled", "");
  }

  return root;
}
