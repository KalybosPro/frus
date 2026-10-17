// When saving a Rust file reloads the running application: the modes of `frus.reloadOnSave`.
// Apart from the extension, so that the rule is tested without VS Code.

/** `never`; by hand (Ctrl+S) only; by hand and with changes only; or every save. */
export type ReloadOnSave = "never" | "manual" | "manualIfDirty" | "all";

/** Whether a save, by hand or not and of a changed file or not, reloads under `mode`. */
export function savesReload(mode: ReloadOnSave, manual: boolean, dirty: boolean): boolean {
  switch (mode) {
    case "never":
      return false;
    case "manual":
      return manual;
    case "manualIfDirty":
      return manual && dirty;
    case "all":
      return true;
  }
}
