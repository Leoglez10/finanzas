// Run: pnpm --dir desktop test   (Node runs TypeScript directly)
import { test } from "node:test";
import assert from "node:assert/strict";
import { parseCents, parseQuick, type QuickContext } from "../src/lib/quick-parse.ts";

const ctx: QuickContext = {
  today: new Date(2026, 10, 10), // martes 10 nov 2026
  history: [
    { payee: "Uber", categoryId: 3, accountId: 1 },
    { payee: "Supermercado La Esquina", categoryId: 2, accountId: 1 },
    { payee: "Empresa SA", categoryId: 9, accountId: 1 },
  ],
  accounts: [{ id: 1, name: "Cuenta principal" }, { id: 2, name: "Tarjeta" }],
  categories: [{ id: 2, name: "Alimentación" }, { id: 3, name: "Transporte" }, { id: 4, name: "Salud" }, { id: 9, name: "Sueldo" }],
  incomeCategoryIds: [9],
};
const day = (d: Date) => [d.getFullYear(), d.getMonth() + 1, d.getDate()].join("-");

test("parseCents accepts only comma thousands and period decimals", () => {
  assert.equal(parseCents("1,250.50"), 125050);
  assert.equal(parseCents("-80"), -8000);
  assert.equal(parseCents("$3,200"), 320000);
  assert.equal(parseCents("0.1"), 10);
  assert.equal(parseCents("1,2"), null);
  assert.equal(parseCents("12.345"), null);
});

test("history decides payee, category and account; words set the date", () => {
  const p = parseQuick("uber 120 ayer", ctx);
  assert.deepEqual([p.kind, p.cents, p.payee, p.categoryId, p.accountId, day(p.date)], ["gasto", 12000, "Uber", 3, 1, "2026-11-9"]);
});

test("prefix matches history and an explicit account wins", () => {
  const p = parseQuick("super 450.50 tarjeta", ctx);
  assert.deepEqual([p.cents, p.payee, p.categoryId, p.accountId], [45050, "Supermercado La Esquina", 2, 2]);
});

test("income from a plus sign or an income category", () => {
  assert.deepEqual([parseQuick("+3,200 sueldo", ctx).kind, parseQuick("+3,200 sueldo", ctx).cents], ["ingreso", 320000]);
  assert.equal(parseQuick("empresa 100", ctx).kind, "ingreso");
});

test("keywords map to the user's categories by name; weekdays and d/m dates", () => {
  const tacos = parseQuick("tacos 85 domingo", ctx);
  assert.deepEqual([tacos.payee, tacos.categoryId, day(tacos.date)], ["Tacos", 2, "2026-11-8"]);
  const pharmacy = parseQuick("farmacia 99 5/11", ctx);
  assert.deepEqual([pharmacy.cents, pharmacy.categoryId, day(pharmacy.date)], [9900, 4, "2026-11-5"]);
});

test("no amount and no category stay empty for the caller to ask", () => {
  const p = parseQuick("regalo para mamá", ctx);
  assert.deepEqual([p.cents, p.payee, p.categoryId], [null, "Regalo mamá", null]);
});
