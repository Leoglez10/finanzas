// Right-click menus on rows: a native menu with the row's actions (Editar, Duplicar, Eliminar…).
import { Menu } from "@tauri-apps/api/menu";

/** `null` draws a separator; `false` is an entry that does not apply here and is left out. */
export type MenuEntry = readonly [label: string, run: () => unknown] | null | false;

/** Drops left-out entries and separators that would come first, last or twice in a row. */
export const tidy = (entries: MenuEntry[]) => entries.filter(x => x !== false)
  .filter((x, i, all) => x || (i > 0 && all[i - 1] && all.slice(i + 1).some(Boolean))) as (readonly [string, () => unknown] | null)[];

export async function showMenu(e: MouseEvent, entries: MenuEntry[]) {
  e.preventDefault();
  // Dev QA scripts read the entries instead: a native menu would wait for a real click.
  const qa = import.meta.env.DEV ? (window as unknown as { __finanzas?: { capture?: boolean; lastMenu?: MenuEntry[] } }).__finanzas : undefined;
  if (qa?.capture) { qa.lastMenu = tidy(entries); return; }
  const menu = await Menu.new({ items: tidy(entries).map(x => x ? { text: x[0], action: () => void x[1]() } : { item: "Separator" as const }) });
  await menu.popup();
}
