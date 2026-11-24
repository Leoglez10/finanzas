// Run: pnpm --dir desktop test
import { test } from "node:test";
import assert from "node:assert/strict";
import { csvRows, detect, isDuplicate, mapRows, parseDate, signature, withSaved, planSheets, defaultSheets, sheetRows, cleanPayee, repeatedInFile, type Role, type Sheet } from "../src/lib/csv.ts";
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

const pick = (text: string) => mapRows(csvRows(text), ctx).map(r => [r.line, r.date, r.payee, r.cents]);

test("title rows above the header are skipped and lines keep their file numbers", () => {
  assert.deepEqual(pick("Estado de cuenta BBVA\nCuenta,0123\n\nFecha,Concepto,Importe\n09/11/2026,Netflix,-350"), [[5, "2026-11-09", "Netflix", -35000]]);
});

test("columns with unknown names are found by their content", () => {
  const d = detect(csvRows("Cuándo,Qué,Cuánto\n09/11/2026,Netflix,-350.00\n10/11/2026,Uber,-80.50"));
  assert.deepEqual(d.roles, ["date", "payee", "amount"]);
  assert.deepEqual(pick("09/11/2026,Netflix,-350.00\n10/11/2026,Uber,-80.50"), [[1, "2026-11-09", "Netflix", -35000], [2, "2026-11-10", "Uber", -8050]]);
  assert.deepEqual(pick("Referencia,Monto,Detalle,Fecha\n0012,-80,Uber,10/11/2026"), [[2, "2026-11-10", "Uber", -8000]]);
});

test("a balance column is never taken as the amount", () => {
  assert.deepEqual(detect(csvRows("Fecha,Descripción,Importe,Saldo\n09/11/2026,Netflix,-350,1000")).roles, ["date", "payee", "amount", "ignore"]);
});

test("date order comes from the values; undecidable columns are flagged", () => {
  const us = detect(csvRows("Date,Description,Amount\n11/25/2026,Netflix,-15.99\n11/03/2026,Uber,-8"));
  assert.equal(us.dateOrder, "mdy");
  assert.deepEqual(mapRows(csvRows("Date,Description,Amount\n11/25/2026,Netflix,-15.99\n11/03/2026,Uber,-8"), ctx, us).map(r => r.date), ["2026-11-25", "2026-11-03"]);
  const unsure = detect(csvRows("fecha,comercio,monto\n03/11/2026,Uber,-8"));
  assert.equal(unsure.dateOrder, "dmy");
  assert.equal(unsure.dateAmbiguous, true);
  assert.equal(detect(csvRows("fecha,comercio,monto\n25/11/2026,Uber,-8")).dateAmbiguous, false);
});

test("month names, timestamps and Excel day numbers are dates", () => {
  assert.equal(parseDate("05-nov-2026"), "2026-11-05");
  assert.equal(parseDate("5 Dic 2026"), "2026-12-05");
  assert.equal(parseDate("05/JAN/26"), "2026-01-05");
  assert.equal(parseDate("2026-11-05 13:45:00"), "2026-11-05");
  assert.equal(parseDate("2026/11/05"), "2026-11-05");
  assert.equal(parseDate("11/25/2026", "mdy"), "2026-11-25");
  assert.equal(parseDate("45658"), "2025-01-01");
});

test("decimal comma, parentheses, trailing minus and currency marks", () => {
  assert.deepEqual(pick("Fecha;Concepto;Importe\n01/11/2026;Renta;-1.234,56\n02/11/2026;Café;45,5").map(r => r[3]), [-123456, 4550]);
  assert.deepEqual(pick("fecha,comercio,monto\n01/11/2026,A,(120.00)\n01/11/2026,B,120.00-\n01/11/2026,C,\"$ -1,200.50\"\n01/11/2026,D,MXN 80").map(r => r[3]), [-12000, -12000, -120050, 8000]);
});

test("a cargo/abono type column sets the sign", () => {
  const text = "Fecha,Concepto,Tipo,Importe\n01/11/2026,Renta,Cargo,8000\n02/11/2026,Nómina,Abono,3200\n03/11/2026,Uber,D,80\n04/11/2026,Reembolso,C,50\n05/11/2026,Comisión,Comisión,12";
  assert.deepEqual(detect(csvRows(text)).roles, ["date", "payee", "type", "amount"]);
  assert.deepEqual(pick(text).map(r => r[3]), [-800000, 320000, -8000, 5000, 1200]);
});

test("statements with purchases in positive are flipped, and the choice can be undone", () => {
  const rows = csvRows("Fecha,Comercio,Importe\n01/11/2026,Amazon,500\n02/11/2026,Oxxo,45\n02/11/2026,Uber,55\n03/11/2026,Pago recibido,-600");
  const d = detect(rows);
  assert.equal(d.flip, true);
  assert.deepEqual(mapRows(rows, ctx, d).map(r => r.cents), [-50000, -4500, -5500, 60000]);
  assert.deepEqual(mapRows(rows, ctx, { ...d, flip: false }).map(r => r.cents), [50000, 4500, 5500, -60000]);
  assert.equal(detect(csvRows("Fecha,Concepto,Importe\n01/11/2026,Netflix,-350\n02/11/2026,Nómina,3200\n03/11/2026,Reembolso,50")).flip, false);
  assert.equal(detect(csvRows("fecha,comercio,monto\n01/11/2026,Netflix,-350\n02/11/2026,Nómina,3200")).flip, false);
});

test("total and balance rows are left out instead of failing", () => {
  assert.deepEqual(pick("Fecha,Concepto,Importe\nSaldo inicial,,1000\n01/11/2026,Uber,-80\nTotal,,-80\n,Saldo final,920"), [[3, "2026-11-01", "Uber", -8000]]);
});

test("a role picked by hand replaces the detected one", () => {
  const rows = csvRows("Fecha,Concepto,Referencia,Importe\n01/11/2026,Uber,Viaje al centro,-80");
  const d = detect(rows);
  assert.deepEqual(mapRows(rows, ctx, { ...d, roles: ["date", "ignore", "payee", "amount"] }).map(r => r.payee), ["Viaje al centro"]);
});

test("a file's layout is remembered by its header names", () => {
  const rows = csvRows("Fecha,Concepto,Referencia,Importe\n01/11/2026,Uber,Viaje,80");
  const sig = signature(rows, detect(rows));
  assert.equal(sig, signature(csvRows(" FECHA ,concepto,Referéncia,importe\n02/11/2026,Oxxo,x,1"), detect(csvRows(" FECHA ,concepto,Referéncia,importe\n02/11/2026,Oxxo,x,1"))));
  assert.equal(signature(csvRows("01/11/2026,Uber,-80"), detect(csvRows("01/11/2026,Uber,-80"))), null);
  const saved = { roles: ["date", "ignore", "payee", "amount"] as Role[], dateOrder: "mdy" as const, decimal: "." as const, flip: false };
  const d = withSaved(detect(rows), saved);
  assert.deepEqual([d.roles, d.dateOrder, d.flip, d.dateAmbiguous], [saved.roles, "mdy", false, false]);
  // A saved layout for a different number of columns is not applied.
  assert.deepEqual(withSaved(detect(rows), { ...saved, roles: ["date", "payee", "amount"] }).roles, detect(rows).roles);
});

test("sheets with movements are offered; the ones sharing the main layout are combined", () => {
  const sheets: Sheet[] = [
    { name: "Resumen", rows: [["Saldo inicial", "1000"], ["Saldo final", "800"]] },
    { name: "Ene", rows: [["Fecha", "Concepto", "Importe"], ["05/01/2026", "Uber", "-80"]] },
    { name: "Feb", rows: [["Fecha", "Concepto", "Importe"], ["05/02/2026", "Oxxo", "-45"], ["06/02/2026", "Renta", "-8000"]] },
    { name: "Tarjeta", rows: [["Date", "Description", "Amount"], ["02/15/2026", "Amazon", "-500"]] },
  ];
  const plans = planSheets(sheets);
  assert.deepEqual(plans.map(p => [p.name, p.usable]), [["Resumen", false], ["Ene", true], ["Feb", true], ["Tarjeta", true]]);
  assert.deepEqual(defaultSheets(plans), ["Ene", "Feb"]);
  const main = plans[2];
  assert.deepEqual(sheetRows(sheets, plans, ["Ene", "Feb", "Tarjeta"], main, ctx).map(r => [r.sheet, r.line, r.date, r.payee]), [
    ["Ene", 2, "2026-01-05", "Uber"], ["Feb", 2, "2026-02-05", "Oxxo"], ["Feb", 3, "2026-02-06", "Renta"], ["Tarjeta", 2, "2026-02-15", "Amazon"],
  ]);
  // Changes made in the panel apply to every sheet with the main layout, each with its own header row.
  const edited = { ...main.detection, roles: ["date", "ignore", "amount"] as Role[] };
  assert.deepEqual(sheetRows([{ name: "Ene", rows: [["Mes enero"], ...sheets[1].rows] }, sheets[2]], planSheets([{ name: "Ene", rows: [["Mes enero"], ...sheets[1].rows] }, sheets[2]]), ["Ene", "Feb"], { ...main, detection: edited }, ctx).map(r => [r.line, r.error]),
    [[3, "Sin comercio"], [2, "Sin comercio"], [3, "Sin comercio"]]);
});

test("bank descriptions lose codes, card digits and prefixes, and match a known payee", () => {
  assert.equal(cleanPayee("COMPRA OXXO CENTRO 1234 GDL MX", ctx), "Oxxo Centro Gdl");
  assert.equal(cleanPayee("POS UBER *TRIP 05/11 REF: 88231234", ctx), "Uber Trip");
  assert.equal(cleanPayee("NETFLIX.COM ****4321 AUT 554433", ctx), "Netflix");
  assert.equal(cleanPayee("SPEI ENVIADO JUAN PEREZ 0012345678", ctx), "Spei Enviado Juan Perez");
  assert.equal(cleanPayee("Café Punta del Cielo", ctx), "Café Punta del Cielo");
  assert.equal(cleanPayee("1234567", ctx), "1234567");
  const [row] = mapRows(csvRows("fecha,comercio,monto\n09/11/2026,NETFLIX.COM ****4321,-350"), ctx);
  assert.deepEqual([row.payee, row.raw, row.categoryId], ["Netflix", "NETFLIX.COM ****4321", 4]);
});

test("rows repeated inside the file are found", () => {
  const rows = mapRows(csvRows("fecha,comercio,monto\n09/11/2026,Uber,-80\n09/11/2026,UBER,-80\n09/11/2026,Uber,-90\n31/13/2026,Uber,-80"), ctx);
  assert.deepEqual(repeatedInFile(rows), [false, true, false, false]);
});

test("a description that continues on the next row is joined to the row above", () => {
  const rows = mapRows(csvRows("Fecha,Concepto,Importe\n09/11/2026,PAGO SERVICIO,-500\n,TELMEX HOGAR,\n10/11/2026,Uber,-80\n,,"), ctx);
  assert.deepEqual(rows.map(r => [r.line, r.payee, r.cents, r.error]), [[2, "Pago Servicio Telmex Hogar", -50000, ""], [4, "Uber", -8000, ""]]);
});
