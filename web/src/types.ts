export interface BatteryStatus {
  percentage: number;
  level: string;
  charging: string;
  externalPower: boolean;
}

export interface SmartShiftState {
  mode: string;
  autoDisengage: number;
  torque: number;
}

export interface HostInfo {
  currentHost: number;
  numHosts: number;
}

export interface Feature {
  id: string;
  index: number;
  name: string;
}

/** HID++ notification from the device (diverted button, scroll, battery, etc.) */
export interface HidppNotification {
  featureIndex: number;
  featureId: number;
  functionId: number;
  params: Uint8Array;
}
