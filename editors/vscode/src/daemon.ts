// `frus daemon`, kept running for the window: the devices, as they come and go.

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

/** The daemon's device list, kept current. */
export class Devices {
  private process: ChildProcess | undefined;
  private devices = new Map<string, Device>();
  private listeners: Array<() => void> = [];
  private nextId = 1;

  /** Starts `frus daemon` and asks it to report every change. */
  start(frus: string): void {
    const daemon = spawn(frus, ["daemon"], { stdio: ["pipe", "pipe", "ignore"], windowsHide: true });
    this.process = daemon;
    readLines(daemon.stdout!, (message) => this.take(message));
    daemon.on("exit", () => {
      if (this.process === daemon) this.process = undefined;
    });
    send(daemon, { id: this.nextId++, method: "device.enable" });
  }

  /** A message of the daemon's. */
  take(message: any): void {
    switch (message.event) {
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
