// Run: pnpm --dir desktop test
import { test } from "node:test";
import assert from "node:assert/strict";
import { presetRange, presetOf, rangeOfParams, inRange } from "../src/lib/period.ts";

const today = new Date(2026, 10, 19); // Thursday, Nov 19 2026

test("presets resolve to inclusive ISO ranges", () => {
  assert.deepEqual(presetRange("todo", today), { from: "", to: "" });
  assert.deepEqual(presetRange("semana", today), { from: "2026-11-16", to: "2026-11-22" });
  assert.deepEqual(presetRange("mes", today), { from: "2026-11-01", to: "2026-11-30" });
  assert.deepEqual(presetRange("mes-pasado", today), { from: "2026-10-01", to: "2026-10-31" });
  assert.deepEqual(presetRange("3m", today), { from: "2026-09-01", to: "2026-11-30" });
  assert.deepEqual(presetRange("anio", today), { from: "2026-01-01", to: "2026-12-31" });
});

test("week starts on Monday even on Sunday", () => {
  assert.deepEqual(presetRange("semana", new Date(2026, 10, 22)), { from: "2026-11-16", to: "2026-11-22" });
});

test("a range maps back to its preset or to custom", () => {
  assert.equal(presetOf({ from: "2026-10-01", to: "2026-10-31" }, today), "mes-pasado");
  assert.equal(presetOf({ from: "", to: "" }, today), "todo");
  assert.equal(presetOf({ from: "2026-10-05", to: "" }, today), "rango");
});

test("route params: mes keeps working, desde/hasta win", () => {
  assert.deepEqual(rangeOfParams(new URLSearchParams("mes=202602")), { from: "2026-02-01", to: "2026-02-28" });
  assert.deepEqual(rangeOfParams(new URLSearchParams("desde=2026-01-10&hasta=2026-03-05")), { from: "2026-01-10", to: "2026-03-05" });
  assert.deepEqual(rangeOfParams(new URLSearchParams("")), { from: "", to: "" });
});

test("inRange treats empty bounds as open", () => {
  assert.equal(inRange("2026-11-01", { from: "2026-11-01", to: "2026-11-30" }), true);
  assert.equal(inRange("2026-12-01", { from: "2026-11-01", to: "2026-11-30" }), false);
  assert.equal(inRange("2020-01-01", { from: "", to: "2026-11-30" }), true);
});

test("month grid is 6 Monday-first weeks covering the month", async () => {
  const { monthGrid } = await import("../src/lib/period.ts");
  const grid = monthGrid(2026, 10); // November 2026 starts on a Sunday
  assert.equal(grid.length, 42);
  assert.equal(grid[0], "2026-10-26");
  assert.equal(grid[6], "2026-11-01");
  assert.equal(grid[35], "2026-11-30");
});
