// Bank CSV or spreadsheet rows → import preview. `detect` reads the file's layout (header row, what each
// column holds, date order, decimal mark, sign) from header names and from the values themselves; the
// preview can override any of it. Amounts are integer cents.
import { norm, parseQuick, type QuickContext } from "./quick-parse.ts";

export interface ImportRow { line: number; date: string | null; payee: string; cents: number | null; categoryId: number | null; error: string }

export type Role = "date" | "payee" | "amount" | "out" | "in" | "type" | "category" | "ignore";
/** How to read a file: `header` is the header row index (-1 when there is none). */
export interface Mapping { header: number; roles: Role[]; dateOrder: DateOrder; decimal: "." | ","; flip: boolean }
/** `dateAmbiguous`: every date fits both orders, so `dateOrder` is a guess worth confirming. */
export interface Detection extends Mapping { dateAmbiguous: boolean }
export type DateOrder = "dmy" | "mdy";

export function csvRows(text: string): string[][] {
  // The separator that shows up most in the first lines (title lines before the header have few).
  const head = text.split("\n", 10).join("\n");
  const delimiter = [",", ";", "\t", "|"].reduce((best, d) => head.split(d).length > head.split(best).length ? d : best);
  const rows: string[][] = [];
  let row: string[] = [], cell = "", quoted = false;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (quoted) { if (c !== '"') cell += c; else if (text[i + 1] === '"') { cell += '"'; i++; } else quoted = false; }
    else if (c === '"') quoted = true;
    else if (c === delimiter) { row.push(cell); cell = ""; }
    else if (c === "\n") { row.push(cell); rows.push(row); row = []; cell = ""; }
    else if (c !== "\r" && c !== "﻿") cell += c;
  }
  if (cell || row.length) { row.push(cell); rows.push(row); }
  // Blank lines stay, so a row's index + 1 is its line in the file.
  return rows;
}

const MONTHS = ["ene|jan", "feb", "mar", "abr|apr", "may", "jun", "jul", "ago|aug", "sep|set", "oct", "nov", "dic|dec"];
const monthOf = (word: string) => MONTHS.findIndex(m => m.split("|").some(p => word.startsWith(p))) + 1;
const NUMERIC_DATE = /^(\d{1,2})[/.-](\d{1,2})[/.-](\d{2}|\d{4})$/;

/** "05/11/2026" (day first unless `order` is "mdy"), "2026-11-05", "05-nov-2026", "5 Dic 2026",
 * a time after the date, or an Excel day number → "2026-11-05"; impossible dates → null. */
export function parseDate(text: string, order: DateOrder = "dmy"): string | null {
  const t = norm(text.trim()).replace(/(t|\s+)\d{1,2}:\d{2}(:\d{2})?.*$/, "");
  const num = t.match(NUMERIC_DATE), iso = t.match(/^(\d{4})[/.-](\d{1,2})[/.-](\d{1,2})$/);
  const named = t.match(/^(\d{1,2})[\s/.-]+([a-z]{3,})\.?[\s/.-]+(\d{2}|\d{4})$/);
  const year = (y: string) => +y < 100 ? 2000 + +y : +y;
  let y = 0, m = 0, d = 0;
  if (num) [y, m, d] = order === "dmy" ? [year(num[3]), +num[2], +num[1]] : [year(num[3]), +num[1], +num[2]];
  else if (iso) [y, m, d] = [+iso[1], +iso[2], +iso[3]];
  else if (named) [y, m, d] = [year(named[3]), monthOf(named[2]), +named[1]];
  else if (/^\d{5}$/.test(t) && +t > 20000 && +t < 80000) {
    // Excel counts days from 1899-12-30.
    const date = new Date(1899, 11, 30 + +t);
    [y, m, d] = [date.getFullYear(), date.getMonth() + 1, date.getDate()];
  }
  if (!y || !m) return null;
  const date = new Date(y, m - 1, d);
  if (date.getFullYear() !== y || date.getMonth() !== m - 1 || date.getDate() !== d) return null;
  return `${y}-${String(m).padStart(2, "0")}-${String(d).padStart(2, "0")}`;
}

const CURRENCY = /^(mxn|usd|eur|\$|€)+|(mxn|usd|eur|\$|€)+$/gi;
/** "-1,200.50", "1.234,56" (with `decimal` ","), "(120.00)", "120.00-", "$ 80", "MXN 80" → cents. */
export function parseMoney(text: string | undefined, decimal: "." | "," = "."): number | null {
  let t = (text ?? "").replace(/\s/g, "").replace(CURRENCY, "");
  if (!t) return null;
  let negative = false;
  if (/^\(.*\)$/.test(t)) { negative = true; t = t.slice(1, -1); }
  if (t.endsWith("-")) { negative = true; t = t.slice(0, -1); }
  if (/^[+-]/.test(t)) { negative ||= t[0] === "-"; t = t.slice(1); }
  t = t.replace(CURRENCY, "");
  t = decimal === "," ? t.replace(/\./g, "").replace(",", ".") : t.replace(/,/g, "");
  if (!/^\d+(\.\d+)?$/.test(t)) return null;
  const cents = Math.round(parseFloat(t) * 100);
  return negative && cents ? -cents : cents;
}

/** -1 for a charge ("Cargo", "D", "Débito"), 1 for a deposit ("Abono", "C", "Crédito"), 0 when unknown. */
function typeSign(text: string): -1 | 0 | 1 {
  const t = norm(text.trim());
  if (/^(cargo|debito|debit|retiro|egreso|compra)/.test(t) || /^(d|dr|-)$/.test(t)) return -1;
  if (/^(abono|credito|credit|deposito|ingreso|devolucion)/.test(t) || /^(c|cr|h|\+)$/.test(t)) return 1;
  return 0;
}

// First match wins, so "Tipo de movimiento" is a type and "Fecha valor" is a date.
const NAMES: [Role, RegExp][] = [
  ["ignore", /saldo|balance|folio|^id$|^#$|^no\.?$/],
  ["type", /^tipo|^type$|naturaleza|^d\/?c$|^dr\/?cr$/],
  ["category", /categoria|category|rubro/],
  ["date", /fecha|date|^dia$|posted/],
  ["out", /cargo|retiro|debito|debit|withdrawal|egreso|salida/],
  ["in", /abono|deposito|credito|credit|ingreso|entrada/],
  ["amount", /monto|importe|cantidad|amount|valor|^total$|^\$$/],
  ["payee", /comercio|descripcion|concepto|description|payee|beneficiario|detalle|establecimiento|narrative|memo|details|movimiento/],
];
const TOTAL = /^(saldo|total|subtotal|balance)\b/;
const blank = (row: string[]) => row.every(c => !c.trim());
const anyDate = (v: string) => !!(parseDate(v, "dmy") ?? parseDate(v, "mdy"));
// Plain numbers are dates only in a column named so (Excel day numbers); otherwise they are amounts.
const looksDate = (v: string) => !/^\d+$/.test(v.trim()) && anyDate(v);

function decimalOf(cells: string[]): "." | "," {
  let comma = 0, dot = 0;
  for (const c of cells) {
    const t = c.replace(/[\s()$€+-]|mxn|usd|eur/gi, "");
    const m = /^[\d.,]+$/.test(t) && t.match(/([.,])(\d+)$/);
    if (!m || m[2].length > 2) continue;
    if (m[1] === ",") comma++; else dot++;
  }
  return comma > dot ? "," : ".";
}

function headerRow(rows: string[][]): number {
  const top = rows.slice(0, 20);
  const scores = top.map(r => r.filter(c => c.trim() && NAMES.some(([, re]) => re.test(norm(c.trim())))).length);
  const best = scores.indexOf(Math.max(...scores));
  if (scores[best] >= 2) return best;
  // No known names: the row right above the first dated row is the header when it holds only text.
  const first = top.findIndex(r => r.some(looksDate));
  const above = first > 0 ? rows.slice(0, first).filter(r => !blank(r)).at(-1) : undefined;
  return above && !above.some(c => looksDate(c) || parseMoney(c) !== null) ? rows.indexOf(above) : -1;
}

export function detect(rows: string[][]): Detection {
  const header = headerRow(rows);
  const names = header >= 0 ? rows[header].map(h => norm(h.trim())) : [];
  const data = rows.slice(header + 1, header + 301).filter(r => !blank(r) && !r.some(c => TOTAL.test(norm(c.trim()))));
  const width = Math.max(names.length, ...data.map(r => r.length));
  const column = (i: number) => data.map(r => (r[i] ?? "").trim()).filter(Boolean);
  const share = (i: number, test: (v: string) => boolean) => { const v = column(i); return v.length ? v.filter(test).length / v.length : 0; };
  const decimal = decimalOf(data.flat());
  const isMoney = (v: string) => parseMoney(v, decimal) !== null;

  const roles: (Role | null)[] = Array.from({ length: width }, (_, i) => {
    const role = NAMES.find(([, re]) => names[i] && re.test(names[i]))?.[0] ?? null;
    // A name the values contradict (a "Fecha" column without dates) is dropped.
    if (role === "date" && share(i, anyDate) < 0.5) return null;
    if ((role === "amount" || role === "out" || role === "in") && share(i, isMoney) < 0.5) return null;
    if (role === "type" && share(i, v => typeSign(v) !== 0) < 0.5) return null;
    return role;
  });
  // Only the first column of each role counts ("Fecha valor" after "Fecha" is ignored).
  roles.forEach((r, i) => { if (r && r !== "ignore" && roles.indexOf(r) < i) roles[i] = "ignore"; });
  const free = () => roles.map((r, i) => r ? -1 : i).filter(i => i >= 0);
  const best = (score: (i: number) => number, min: number) => free().reduce<[number, number]>((b, i) => score(i) > b[1] ? [i, score(i)] : b, [-1, min - 1e-9])[0];

  if (!roles.includes("date")) { const i = best(i => share(i, looksDate), 0.6); if (i >= 0) roles[i] = "date"; }
  if (!roles.some(r => r === "amount" || r === "out" || r === "in")) {
    // Decimals or a sign make a column look more like money than an id or a reference number.
    const i = best(i => share(i, isMoney) + share(i, v => /[.,]\d{1,2}\)?-?$|^[-(]/.test(v)) / 2, 0.6);
    if (i >= 0) roles[i] = "amount";
  }
  if (roles.includes("amount") && !roles.includes("type")) { const i = best(i => share(i, v => typeSign(v) !== 0), 0.8); if (i >= 0) roles[i] = "type"; }
  if (!roles.includes("payee")) {
    const i = best(i => { const v = column(i); return v.length ? share(i, x => /[a-z]/i.test(x) && !looksDate(x)) * new Set(v).size / v.length : 0; }, 0.3);
    if (i >= 0) roles[i] = "payee";
  }
  const done = roles.map(r => r ?? "ignore");

  const dateAt = done.indexOf("date");
  const parts = column(dateAt).map(v => v.match(NUMERIC_DATE)).filter(m => !!m);
  const dateOrder: DateOrder = parts.some(m => +m[1] > 12) || !parts.some(m => +m[2] > 12) ? "dmy" : "mdy";
  const dateAmbiguous = parts.length > 0 && !parts.some(m => +m[1] > 12 || +m[2] > 12);
  // Card statements list purchases as positive: when most amounts are positive, read them as charges.
  const amounts = done.includes("type") ? [] : column(done.indexOf("amount")).map(v => parseMoney(v, decimal) ?? 0).filter(Boolean);
  const flip = amounts.filter(c => c > 0).length > amounts.length / 2;
  return { header, roles: done, dateOrder, decimal, flip, dateAmbiguous };
}

export function mapRows(rows: string[][], ctx: QuickContext, m: Mapping = detect(rows)): ImportRow[] {
  const at = (role: Role) => m.roles.indexOf(role);
  const [iDate, iPayee, iAmount, iOut, iIn, iType, iCategory] = (["date", "payee", "amount", "out", "in", "type", "category"] as Role[]).map(at);
  const money = (v: string | undefined) => parseMoney(v, m.decimal);
  const out: ImportRow[] = [];
  rows.forEach((r, index) => {
    if (index <= m.header || blank(r)) return;
    const date = parseDate(r[iDate] ?? "", m.dateOrder);
    if (!date && r.some(c => TOTAL.test(norm(c.trim())))) return;
    const payee = (r[iPayee] ?? "").trim();
    let cents: number | null = null;
    if (iAmount >= 0) {
      cents = money(r[iAmount]);
      const sign = iType >= 0 ? typeSign(r[iType] ?? "") : 0;
      if (cents !== null && sign) cents = sign * Math.abs(cents);
      else if (cents !== null && m.flip && iType < 0) cents = -cents;
    } else {
      const debit = money(r[iOut]), credit = money(r[iIn]);
      cents = debit || credit ? (credit ? Math.abs(credit) : 0) - (debit ? Math.abs(debit) : 0) : null;
    }
    const named = iCategory >= 0 ? ctx.categories.find(c => norm(c.name) === norm((r[iCategory] ?? "").trim())) : undefined;
    const categoryId = named?.id ?? (payee ? parseQuick(payee, ctx).categoryId : null);
    const error = !date ? "Fecha no válida" : !cents ? "Monto no válido" : !payee ? "Sin comercio" : "";
    out.push({ line: index + 1, date, payee, cents, categoryId, error });
  });
  return out;
}

/** Same account, day, amount and payee as an existing movement. */
export const isDuplicate = (row: ImportRow, existing: { date: string; amount: number; payeeName: string | null }[]) =>
  existing.some(e => e.date === row.date && e.amount === row.cents && norm(e.payeeName ?? "") === norm(row.payee));
