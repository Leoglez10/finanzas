// Ajustes › Atajos: the registered commands with a shortcut, plus the editing keys.
import { COMMANDS } from "./commands.svelte";
import { keyLabel } from "./keys";

export const SHORTCUTS: [string, string][] = [
  ...COMMANDS.filter(c => c.keys).map(c => [c.label, keyLabel(c.keys)] as [string, string]),
  ["Guardar en un campo o diálogo", keyLabel("↩")],
  ["Cerrar un diálogo o cancelar una edición", "esc"],
];
