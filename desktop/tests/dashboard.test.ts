// Run: pnpm --dir desktop test
import { test } from "node:test";
import assert from "node:assert/strict";
import { WIDGETS, readLayout, moveTo, toggle, resize } from "../src/lib/dashboard.ts";

const ids = (l: { id: string }[]) => l.map(w => w.id);

test("nothing saved: every widget, visible, in the default order", () => {
  const layout = readLayout(null);
  assert.deepEqual(ids(layout), WIDGETS.map(w => w.id));
  assert.ok(layout.every(w => !w.hidden));
});

test("a saved layout keeps its order, drops unknown ids and appends new widgets", () => {
  const layout = readLayout(JSON.stringify([{ id: "chart", hidden: true }, { id: "gone", hidden: false }, { id: "hero", hidden: false }]));
  assert.deepEqual(ids(layout).slice(0, 2), ["chart", "hero"]);
  assert.equal(layout[0].hidden, true);
  assert.equal(layout.length, WIDGETS.length);
  assert.ok(!ids(layout).includes("gone"));
});

test("broken JSON falls back to the default", () => {
  assert.deepEqual(ids(readLayout("{nope")), WIDGETS.map(w => w.id));
});

test("moveTo puts a widget where another one is", () => {
  const base = readLayout(null);
  const [a, b, c] = ids(base);
  assert.deepEqual(ids(moveTo(base, a, c)).slice(0, 3), [b, c, a]);
  assert.deepEqual(ids(moveTo(base, c, a)).slice(0, 3), [c, a, b]);
  assert.deepEqual(ids(moveTo(base, a, a)), ids(base));
});

test("toggle hides and shows one widget", () => {
  const base = readLayout(null);
  const hidden = toggle(base, "kpis");
  assert.equal(hidden.find(w => w.id === "kpis")!.hidden, true);
  assert.equal(toggle(hidden, "kpis").find(w => w.id === "kpis")!.hidden, false);
});

test("each widget starts at its default width; saved widths are kept, bad ones fixed", () => {
  assert.deepEqual(readLayout(null).map(s => s.span), WIDGETS.map(w => w.span));
  const saved = readLayout(JSON.stringify([{ id: "chart", hidden: false, span: 6 }, { id: "hero", hidden: false, span: 99 }, { id: "kpis", hidden: false }]));
  assert.deepEqual(saved.slice(0, 3).map(s => s.span), [6, WIDGETS.find(w => w.id === "hero")!.span, WIDGETS.find(w => w.id === "kpis")!.span]);
});

test("resize sets a width between 2 and 6 columns", () => {
  const base = readLayout(null);
  assert.equal(resize(base, "chart", 5).find(s => s.id === "chart")!.span, 5);
  assert.equal(resize(base, "chart", 1).find(s => s.id === "chart")!.span, 2);
  assert.equal(resize(base, "chart", 9).find(s => s.id === "chart")!.span, 6);
});
