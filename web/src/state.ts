import type { Device } from "./device";

export type Section = "dpi" | "scroll" | "buttons" | "host" | "info";

export interface AppState {
  section: Section;
  device: Device | null;
  sections: SupportedSections;
}

export interface SupportedSections {
  dpi: boolean;
  scroll: boolean;
  buttons: boolean;
  host: boolean;
  battery: boolean;
  thumbwheel: boolean;
  firmware: boolean;
  friendlyName: boolean;
  hostsInfo: boolean;
}

type Listener = () => void;

const listeners: Listener[] = [];

export const state: AppState = {
  section: "dpi",
  device: null,
  sections: {
    dpi: false,
    scroll: false,
    buttons: false,
    host: false,
    battery: false,
    thumbwheel: false,
    firmware: false,
    friendlyName: false,
    hostsInfo: false,
  },
};

export function setSection(section: Section): void {
  state.section = section;
  notify();
}

export function setDevice(device: Device, sections: SupportedSections): void {
  state.device = device;
  state.sections = sections;
  // Default to first available section.
  if (sections.dpi) state.section = "dpi";
  else if (sections.scroll) state.section = "scroll";
  else if (sections.buttons) state.section = "buttons";
  else state.section = "info";
  notify();
}

export function subscribe(fn: Listener): void {
  listeners.push(fn);
}

function notify(): void {
  for (const fn of listeners) {
    fn();
  }
}
