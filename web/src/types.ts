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
