import { el } from "../dom";

interface ButtonHotspot {
  readonly cid: number;
  readonly name: string;
  readonly x: number;
  readonly y: number;
}

const MX_MASTER_3S_BUTTONS: readonly ButtonHotspot[] = [
  { cid: 82, name: "Middle Click", x: 71, y: 15 },
  { cid: 196, name: "Mode Shift", x: 81, y: 34 },
  { cid: 86, name: "Forward", x: 35, y: 43 },
  { cid: 195, name: "Gesture", x: 8, y: 58 },
  { cid: 83, name: "Back", x: 45, y: 60 },
] as const;

export interface DeviceImageCard {
  readonly root: HTMLElement;
  readonly setActiveButton: (cid: number | null) => void;
}

export function createDeviceImageCard(
  onButtonClick?: (cid: number, name: string) => void,
): DeviceImageCard {
  const container = el("div", { class: "device-image-container" });

  const imageWrapper = el("div", { class: "device-image-wrapper" });

  const img = el("img", {
    src: "/logi-re/devices/assets/mx-master-3s/side.png",
    alt: "MX Master 3S",
    class: "device-image",
    draggable: "false",
  });

  imageWrapper.append(img);

  // Add button hotspots.
  const hotspotEls = new Map<number, HTMLElement>();
  for (const btn of MX_MASTER_3S_BUTTONS) {
    const dot = el(
      "button",
      {
        class: "hotspot",
        title: btn.name,
        style: `left: ${String(btn.x)}%; top: ${String(btn.y)}%`,
        "data-cid": String(btn.cid),
      },
      el("span", { class: "hotspot-ring" }),
    );

    dot.addEventListener("click", () => {
      if (onButtonClick) {
        onButtonClick(btn.cid, btn.name);
      }
    });

    hotspotEls.set(btn.cid, dot);
    imageWrapper.append(dot);
  }

  container.append(imageWrapper);

  return {
    root: container,
    setActiveButton(cid: number | null): void {
      for (const [id, dot] of hotspotEls) {
        dot.classList.toggle("active", id === cid);
      }
    },
  };
}
