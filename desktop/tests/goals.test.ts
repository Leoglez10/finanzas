// Run: pnpm --dir desktop test
import { test } from "node:test";
import assert from "node:assert/strict";
import { KINDS, kindOf, planText } from "../src/lib/goals.ts";

test("every goal type has its icon and its own wording", () => {
  assert.deepEqual(KINDS.map(k => k.id), ["compra", "viaje", "experiencia", "fondo"]);
  assert.equal(kindOf("viaje").namePlaceholder, "Viaje a Cancún");
  assert.equal(kindOf("viaje").done, "Ya viajé");
  assert.equal(kindOf(undefined).id, "compra");
});

test("the plan reads as what to set aside each month", () => {
  assert.equal(planText({ targetDate: "2027-06-15", remaining: 840000, monthsLeft: 7, monthly: 120000, overdue: false }),
    "Aparta $1,200.00 al mes · 7 meses, hasta el 15 jun 2027");
  assert.equal(planText({ targetDate: "2026-11-30", remaining: 50000, monthsLeft: 1, monthly: 50000, overdue: false }),
    "Aparta $500.00 este mes · hasta el 30 nov 2026");
  assert.equal(planText({ targetDate: "2026-10-01", remaining: 50000, monthsLeft: 0, monthly: 50000, overdue: true }),
    "Vencida el 1 oct 2026 · faltan $500.00");
  assert.equal(planText({ targetDate: "2027-06-15", remaining: 0, monthsLeft: 7, monthly: 0, overdue: false }),
    "Meta alcanzada antes del 15 jun 2027");
});
