// One registry for the native menu (src-tauri/src/menu.rs uses the same ids and owns the shortcuts),
// the ⌘K palette and Ajustes › Atajos.
import { invoke } from "@tauri-apps/api/core";
import { app, go, updateSettings } from "./app.svelte";
import { openAccount, openEntry, openImport, openSchedule, openWish } from "./dialogs.svelte";
import { pickCsv } from "./api";
import { toast, undo } from "./undo.svelte";

export interface Command { id: string; label: string; group: string; icon: string; keys: string; run: () => unknown }

export const palette = $state({ open: false, text: "" });
export const openPalette = (text = "") => Object.assign(palette, { open: true, text });

// Collapsed to icons by choice (remembered on this computer); narrow windows collapse it on their own.
export const sidebar = $state({ collapsed: localStorage.getItem("sidebar-collapsed") === "1" });
export function toggleSidebar() {
  sidebar.collapsed = !sidebar.collapsed;
  localStorage.setItem("sidebar-collapsed", sidebar.collapsed ? "1" : "0");
}

const isDark = () => app.settings?.dark ?? matchMedia("(prefers-color-scheme: dark)").matches;

export const COMMANDS: Command[] = [
  { id: "go-resumen", label: "Ir a Resumen", group: "Navegar", icon: "home", keys: "⌘1", run: () => go("resumen") },
  { id: "go-movimientos", label: "Ir a Movimientos", group: "Navegar", icon: "list", keys: "⌘2", run: () => go("movimientos") },
  { id: "go-presupuesto", label: "Ir a Presupuesto", group: "Navegar", icon: "pie", keys: "⌘3", run: () => go("presupuesto") },
  { id: "go-proximos", label: "Ir a Próximos pagos", group: "Navegar", icon: "cal", keys: "⌘4", run: () => go("proximos") },
  // Off until enabled in Ajustes: then the command opens that switch instead.
  { id: "go-deudas", label: "Ir a Deudas", group: "Navegar", icon: "card", keys: "", run: () => go(app.settings?.debts_page ? "deudas" : "ajustes/deudas") },
  { id: "go-deseos", label: "Ir a Deseos", group: "Navegar", icon: "gift", keys: "⌘5", run: () => go("deseos") },
  { id: "go-ajustes", label: "Abrir Ajustes", group: "Navegar", icon: "gear", keys: "⌘,", run: () => go("ajustes") },
  { id: "new-gasto", label: "Nuevo gasto", group: "Acciones", icon: "down", keys: "⌘N", run: () => openEntry("gasto") },
  { id: "new-ingreso", label: "Nuevo ingreso", group: "Acciones", icon: "up", keys: "⇧⌘N", run: () => openEntry("ingreso") },
  { id: "quick", label: "Captura rápida", group: "Acciones", icon: "bolt", keys: "⌥⇧F", run: () => invoke("show_quick") },
  { id: "new-wish", label: "Nueva meta o deseo", group: "Acciones", icon: "gift", keys: "", run: () => openWish() },
  { id: "new-account", label: "Nueva cuenta", group: "Acciones", icon: "card", keys: "", run: () => openAccount() },
  { id: "new-schedule", label: "Programar pago", group: "Acciones", icon: "repeat", keys: "", run: () => openSchedule() },
  { id: "import", label: "Importar CSV, Excel o PDF…", group: "Acciones", icon: "upload", keys: "⌘I", run: () => pickCsv().then(f => f && openImport(f)).catch(e => toast(String(e))) },
  { id: "search", label: "Buscar movimientos", group: "Editar", icon: "search", keys: "⌘F", run: () => { go("movimientos"); setTimeout(() => document.querySelector<HTMLInputElement>("#main input[type=search]")?.focus()); } },
  { id: "undo", label: "Deshacer el último cambio", group: "Editar", icon: "undo", keys: "⌘Z", run: () => undo() },
  { id: "sidebar", label: "Mostrar u ocultar la barra lateral", group: "Ver", icon: "sidebar", keys: "⌘S", run: () => toggleSidebar() },
  { id: "theme", label: "Cambiar entre claro y oscuro", group: "Ver", icon: "moon", keys: "⇧⌘L", run: () => updateSettings({ dark: !isDark() }) },
  { id: "palette", label: "Paleta de comandos", group: "Ventana", icon: "command", keys: "⌘K", run: () => openPalette() },
];

export function runCommand(id: string) {
  const command = COMMANDS.find(c => c.id === id);
  if (!command) return;
  // A menu shortcut while a dialog is open would stack windows; the palette is the exception.
  const open = document.querySelector<HTMLDialogElement>("dialog[open]");
  if (open && id !== "palette") return;
  open?.close();
  void command.run();
}
