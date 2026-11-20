// Run: pnpm --dir desktop test
import { test } from "node:test";
import assert from "node:assert/strict";
import { csvRows, isDuplicate, mapRows, parseDate } from "../src/lib/csv.ts";
import type { QuickContext } from "../src/lib/quick-parse.ts";

const ctx: QuickContext = {
  today: new Date(2026, 10, 10),
  history: [{ payee: "Netflix", categoryId: 4, accountId: 1 }],
  accounts: [{ id: 1, name: "Cuenta principal" }],
  categories: [{ id: 2, name: "Alimentación" }, { id: 3, name: "Transporte" }, { id: 4, name: "Ocio" }],
  incomeCategoryIds: [],
};

test("quoted fields keep their commas and escaped quotes", () => {
  assert.deepEqual(csvRows('fecha,comercio,monto\r\n09/11/2026,"Oxxo, centro","-1,200.50"\n08/11/2026,"Dice ""hola""",-5\n'),
    [["fecha", "comercio", "monto"], ["09/11/2026", "Oxxo, centro", "-1,200.50"], ["08/11/2026", 'Dice "hola"', "-5"]]);
});

test("semicolon files are detected from the first line", () => {
  assert.deepEqual(csvRows("Fecha;Concepto;Importe\n01/11/2026;Uber;-80"), [["Fecha", "Concepto", "Importe"], ["01/11/2026", "Uber", "-80"]]);
});

test("dates are day-first; impossible dates are rejected", () => {
  assert.equal(parseDate("5/11/26"), "2026-11-05");
  assert.equal(parseDate("05-11-2026"), "2026-11-05");
  assert.equal(parseDate("2026-11-05"), "2026-11-05");
  assert.equal(parseDate("31/13/2026"), null);
  assert.equal(parseDate("30/02/2026"), null);
});

test("amount column, categories by name, history or keyword, and row errors", () => {
  const rows = mapRows(csvRows("fecha,comercio,monto,categoria\n09/11/2026,Netflix,-350.00,\n08/11/2026,Gasolinera Shell,-820.50,\n07/11/2026,Tienda,-64.00,alimentacion\n31/13/2026,Rota,-10,\n06/11/2026,Sin monto,,"), ctx);
  assert.deepEqual(rows.map(r => [r.line, r.date, r.cents, r.categoryId, r.error]), [
    [2, "2026-11-09", -35000, 4, ""],
    [3, "2026-11-08", -82050, 3, ""],
    [4, "2026-11-07", -6400, 2, ""],
    [5, null, -1000, null, "Fecha no válida"],
    [6, "2026-11-06", null, null, "Monto no válido"],
  ]);
});

test("cargo/abono columns become one signed amount", () => {
  const rows = mapRows(csvRows("Fecha,Descripción,Cargo,Abono\n01/11/2026,Renta,\"8,000.00\",\n02/11/2026,Nómina,,\"3,200.00\""), ctx);
  assert.deepEqual(rows.map(r => [r.payee, r.cents]), [["Renta", -800000], ["Nómina", 320000]]);
});

test("duplicates match day, amount and payee ignoring case and accents", () => {
  const [row] = mapRows(csvRows("fecha,comercio,monto\n09/11/2026,NETFLIX,-350"), ctx);
  assert.equal(isDuplicate(row, [{ date: "2026-11-09", amount: -35000, payeeName: "Netflix" }]), true);
  assert.equal(isDuplicate(row, [{ date: "2026-11-10", amount: -35000, payeeName: "Netflix" }]), false);
});
