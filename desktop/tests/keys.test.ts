// Run: pnpm --dir desktop test
import { test } from "node:test";
import assert from "node:assert/strict";
import { comboOf, keyLabel } from "../src/lib/keys.ts";

test("shortcut labels keep Mac symbols on Mac and spell out Ctrl elsewhere", () => {
  assert.equal(keyLabel("⌘K", true), "⌘K");
  assert.equal(keyLabel("⌘K", false), "Ctrl+K");
  assert.equal(keyLabel("⇧⌘N", false), "Ctrl+Shift+N");
  assert.equal(keyLabel("⌥⇧F", false), "Alt+Shift+F");
  assert.equal(keyLabel("⌘↩", false), "Ctrl+Enter");
  assert.equal(keyLabel("esc", false), "esc");
});

test("a Ctrl key press maps to the registry's shortcut string", () => {
  const press = (key: string, extra = {}) => ({ key, ctrlKey: true, metaKey: false, shiftKey: false, altKey: false, ...extra });
  assert.equal(comboOf(press("k")), "⌘K");
  assert.equal(comboOf(press("N", { shiftKey: true })), "⇧⌘N");
  assert.equal(comboOf(press(",")), "⌘,");
  assert.equal(comboOf(press("f", { altKey: true })), null);
  assert.equal(comboOf({ ...press("k"), ctrlKey: false }), null);
});
