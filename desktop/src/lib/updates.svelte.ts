// In-app updates: GitHub Releases publishes a signed latest.json; the app downloads, verifies and
// installs it, then restarts. Homebrew installs the app; its cask is marked auto_updates.
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

type Status = "idle" | "checking" | "none" | "available" | "downloading" | "ready" | "error";
export const updates = $state({ status: "idle" as Status, version: "", notes: "", progress: 0, error: "" });
let pending: Update | null = null;

/** `quiet` (the automatic check at launch) keeps failures out of sight; offline is normal. */
export async function checkUpdates(quiet = false) {
  if (updates.status === "checking" || updates.status === "downloading") return;
  updates.status = "checking"; updates.error = "";
  try {
    pending = await check();
    if (pending) Object.assign(updates, { status: "available", version: pending.version, notes: pending.body ?? "" });
    else updates.status = "none";
  } catch (e) {
    updates.status = quiet ? "idle" : "error";
    updates.error = `No pude revisar si hay actualizaciones: ${e}`;
  }
}

export async function installUpdate() {
  if (!pending) return;
  let total = 0, received = 0;
  Object.assign(updates, { status: "downloading", progress: 0 });
  try {
    await pending.downloadAndInstall(event => {
      if (event.event === "Started") total = event.data.contentLength ?? 0;
      if (event.event === "Progress") { received += event.data.chunkLength; updates.progress = total ? received / total : 0; }
    });
    updates.status = "ready";
  } catch (e) {
    Object.assign(updates, { status: "error", error: `No se pudo instalar la actualización: ${e}` });
  }
}

export const restart = () => relaunch();
