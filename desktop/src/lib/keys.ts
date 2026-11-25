// Shortcuts are written once with Mac symbols (⌘K, ⇧⌘N); Windows and Linux show and match them as Ctrl.
export const isMac = typeof navigator !== "undefined" && /Mac/.test(navigator.platform);

const NAMES: Record<string, string> = { "⌘": "Ctrl+", "⇧": "Shift+", "⌥": "Alt+", "↩": "Enter" };

export function keyLabel(keys: string, mac = isMac): string {
  if (mac || !/[⌘⇧⌥]/.test(keys)) return keys;
  // Ctrl first, as Windows writes it: ⇧⌘N → Ctrl+Shift+N.
  const ctrl = keys.includes("⌘") ? "Ctrl+" : "";
  return ctrl + [...keys.replace("⌘", "")].map(c => NAMES[c] ?? c).join("");
}

type Press = Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "shiftKey" | "altKey">;

/** The registry string for a Ctrl press (⌘K, ⇧⌘N), or null when it is not one. */
export function comboOf(e: Press): string | null {
  if (!e.ctrlKey || e.metaKey || e.altKey || e.key.length !== 1) return null;
  return (e.shiftKey ? "⇧" : "") + "⌘" + e.key.toUpperCase();
}
