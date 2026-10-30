// `frus daemon`, kept running for the window: the devices, as they come and go, the emulators that
// can be started or made, and what the application builds for.

import { ChildProcess, spawn } from "child_process";

import { readLines, send } from "./lines";

/** A device as `frus daemon` describes it. */
export interface Device {
  id: string;
  name: string;
  platform: string;
  category: "desktop" | "mobile" | "web";
  detail: string;
  emulator: boolean;
  available: boolean;
  unavailableReason: string | null;
}

/** Whether `device` is a phone or a tablet simulated in a window of this computer (`frus run
 *  -d iphone-15`): the daemon gives it this computer's platform and category, and an id of its
 *  own, where this computer's id is its platform. */
export function isSimulated(device: Pick<Device, "id" | "platform" | "category">): boolean {
  return device.category === "desktop" && device.id !== device.platform;
}

/** An emulator that can be started, as `emulator.getEmulators` lists it. */
export interface Emulator {
  id: string;
  name: string;
  category: string;
  platformType: string;
}

/** What `daemon.getSupportedPlatforms` says of one platform. */
export interface PlatformSupport {
  isSupported: boolean;
  reasons?: Array<{ reasonText: string; fixText: string }>;
}

/** What the daemon says on its own: a step it is taking, or something to show now. */
export interface DaemonMessage {
  kind: "log" | "show";
  level: string;
  message: string;
}

/** The daemon's device list, kept current, and its other questions. */
export class Devices {
  private process: ChildProcess | undefined;
  private devices = new Map<string, Device>();
  private listeners: Array<() => void> = [];
  private messageListeners: Array<(message: DaemonMessage) => void> = [];
  private pending = new Map<number, { resolve(value: any): void; reject(error: Error): void }>();
  private nextId = 1;

  /** Starts `frus daemon` and asks it to report every change. */
  start(frus: string): void {
    const daemon = spawn(frus, ["daemon"], { stdio: ["pipe", "pipe", "ignore"], windowsHide: true });
    this.process = daemon;
    readLines(daemon.stdout!, (message) => this.take(message));
    daemon.on("exit", () => {
      if (this.process === daemon) this.process = undefined;
      for (const { reject } of this.pending.values()) reject(new Error("frus daemon stopped"));
      this.pending.clear();
    });
    send(daemon, { id: this.nextId++, method: "device.enable" });
  }

  /** Stops the daemon running and starts `frus`'s: a new install. The list starts again from
   *  what the new one reports. */
  restart(frus: string): void {
    this.dispose();
    this.process = undefined;
    this.devices.clear();
    this.listeners.forEach((listener) => listener());
    this.start(frus);
  }

  /** Asks the daemon, and waits for its answer. */
  request<T = any>(method: string, params: object = {}): Promise<T> {
    const process = this.process;
    if (!process) return Promise.reject(new Error("frus daemon is not running"));
    const id = this.nextId++;
    return new Promise<T>((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      send(process, { id, method, params });
    });
  }

  /** The emulators that can be started. */
  emulators(): Promise<Emulator[]> {
    return this.request<Emulator[]>("emulator.getEmulators");
  }

  /** Starts an emulator; it arrives as a device once it is up. */
  launchEmulator(id: string, coldBoot = false): Promise<void> {
    return this.request("emulator.launch", { emulatorId: id, coldBoot });
  }

  /** Makes a new Android emulator, installing what it needs. */
  createEmulator(name?: string): Promise<{ success: boolean; emulatorName: string | null; error: string | null }> {
    return this.request("emulator.create", name ? { name } : {});
  }

  /** What the application in `root` builds for. */
  supportedPlatforms(root: string): Promise<{ platforms: string[]; platformTypes: Record<string, PlatformSupport> }> {
    return this.request("daemon.getSupportedPlatforms", { projectRoot: root });
  }

  onMessage(listener: (message: DaemonMessage) => void): { dispose(): void } {
    this.messageListeners.push(listener);
    return { dispose: () => (this.messageListeners = this.messageListeners.filter((l) => l !== listener)) };
  }

  /** A message of the daemon's. */
  take(message: any): void {
    if (typeof message.id === "number" && this.pending.has(message.id)) {
      const waiting = this.pending.get(message.id)!;
      this.pending.delete(message.id);
      if ("error" in message && message.error !== undefined && message.error !== null) {
        waiting.reject(new Error(String(message.error)));
      } else {
        waiting.resolve(message.result);
      }
      return;
    }
    switch (message.event) {
      case "daemon.logMessage":
      case "daemon.showMessage": {
        const said: DaemonMessage = {
          kind: message.event === "daemon.logMessage" ? "log" : "show",
          level: message.params?.level ?? "info",
          message: message.params?.message ?? "",
        };
        this.messageListeners.forEach((listener) => listener(said));
        return;
      }
      case "device.added":
        this.devices.set(message.params.id, message.params);
        break;
      case "device.removed":
        this.devices.delete(message.params.id);
        break;
      default:
        return;
    }
    this.listeners.forEach((listener) => listener());
  }

  /** The devices now, the desktop first, then phones, then browsers. */
  list(): Device[] {
    const order = { desktop: 0, mobile: 1, web: 2 };
    return [...this.devices.values()].sort(
      (a, b) => order[a.category] - order[b.category] || a.name.localeCompare(b.name),
    );
  }

  get(id: string): Device | undefined {
    return this.devices.get(id);
  }

  onDidChange(listener: () => void): { dispose(): void } {
    this.listeners.push(listener);
    return { dispose: () => (this.listeners = this.listeners.filter((l) => l !== listener)) };
  }

  dispose(): void {
    if (this.process) {
      send(this.process, { id: this.nextId++, method: "daemon.shutdown" });
      const process = this.process;
      setTimeout(() => process.kill(), 2000).unref?.();
    }
  }
}
