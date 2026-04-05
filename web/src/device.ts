import init, { WasmDevice } from "hidpp-web";
import type { BatteryStatus, SmartShiftState, HostInfo, Feature } from "./types";
import { log, logError } from "./log";

let initialized = false;

async function ensureInit(): Promise<void> {
  if (!initialized) {
    log("Loading WASM module...");
    await init();
    initialized = true;
    log("WASM module loaded.");
  }
}

export function isSupported(): boolean {
  try {
    const supported = typeof navigator !== "undefined" && "hid" in navigator;
    log(`WebHID supported: ${supported}`);
    return supported;
  } catch {
    return false;
  }
}

export async function connect(): Promise<Device> {
  await ensureInit();
  if (!WasmDevice.isSupported()) {
    throw new Error("WebHID not available. Use Chrome or Edge.");
  }
  log("Requesting device (browser picker)...");
  try {
    const raw = await WasmDevice.connect();
    log(`Connected: ${raw.name} (${raw.featureCount} features)`);
    return new Device(raw);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    logError(`connect failed: ${msg}`);
    throw e;
  }
}

export class Device {
  readonly #raw: WasmDevice;

  constructor(raw: WasmDevice) {
    this.#raw = raw;
  }

  get name(): string {
    return this.#raw.name;
  }

  get featureCount(): number {
    return this.#raw.featureCount;
  }

  getFeatures(): Feature[] {
    const features = this.#raw.getFeatures() as Feature[];
    log(`getFeatures: ${features.length} features`);
    return features;
  }

  async getBattery(): Promise<BatteryStatus> {
    log("getBattery...");
    const result = (await this.#raw.getBattery()) as BatteryStatus;
    log(`getBattery: ${result.percentage}% ${result.level} ${result.charging}`);
    return result;
  }

  async getDpi(): Promise<number> {
    log("getDpi...");
    const dpi = await this.#raw.getDpi();
    log(`getDpi: ${dpi}`);
    return dpi;
  }

  async setDpi(dpi: number): Promise<number> {
    log(`setDpi(${dpi})...`);
    const applied = await this.#raw.setDpi(dpi);
    log(`setDpi: applied ${applied}`);
    return applied;
  }

  async getSmartShift(): Promise<SmartShiftState> {
    log("getSmartShift...");
    const state = (await this.#raw.getSmartShift()) as SmartShiftState;
    log(`getSmartShift: ${state.mode} disengage=${state.autoDisengage} torque=${state.torque}`);
    return state;
  }

  async setSmartShift(
    mode: string,
    autoDisengage: number,
    torque: number,
  ): Promise<SmartShiftState> {
    log(`setSmartShift(${mode}, ${autoDisengage}, ${torque})...`);
    const result = (await this.#raw.setSmartShift(mode, autoDisengage, torque)) as SmartShiftState;
    log(`setSmartShift: ${result.mode}`);
    return result;
  }

  async getHostInfo(): Promise<HostInfo> {
    log("getHostInfo...");
    const info = (await this.#raw.getHostInfo()) as HostInfo;
    log(`getHostInfo: host ${info.currentHost + 1} of ${info.numHosts}`);
    return info;
  }

  async getFirmware(): Promise<unknown[]> {
    log("getFirmware...");
    const fw = (await this.#raw.getFirmware()) as unknown[];
    log(`getFirmware: ${fw.length} entities`);
    return fw;
  }

  async getButtons(): Promise<unknown[]> {
    log("getButtons...");
    const btns = (await this.#raw.getButtons()) as unknown[];
    log(`getButtons: ${btns.length} controls`);
    return btns;
  }
}
