// Run: pnpm --dir desktop test
import { test } from "node:test";
import assert from "node:assert/strict";
import { tidy, type MenuEntry } from "../src/lib/contextmenu.ts";

test("left-out entries vanish and separators never lead, trail or repeat", () => {
  const a = () => 0;
  const labels = (entries: MenuEntry[]) => tidy(entries).map(x => x ? x[0] : "—");
  assert.deepEqual(labels([false, false, false && null, ["Editar", a], false]), ["Editar"]);
  assert.deepEqual(labels([null, ["Editar", a], null, null, ["Eliminar", a], null]), ["Editar", "—", "Eliminar"]);
  assert.deepEqual(labels([["Editar", a], ["Duplicar", a], false, false, null, ["Eliminar", a]]), ["Editar", "Duplicar", "—", "Eliminar"]);
  assert.deepEqual(labels([["Editar", a], null, false]), ["Editar"]);
});
