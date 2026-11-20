// Bank CSV → import preview. Handles quoted fields, "," or ";" delimiters, Spanish or English headers,
// and banks that split money into "cargo"/"abono" columns. Amounts are integer cents.
import { norm, parseCents, parseQuick, type QuickContext } from "./quick-parse.ts";

export interface ImportRow { line: number; date: string | null; payee: string; cents: number | null; categoryId: number | null; error: string }

export function csvRows(text: string): string[][] {
  const firstLine = text.slice(0, text.indexOf("\n") >>> 0);
  const delimiter = (firstLine.match(/;/g)?.length ?? 0) > (firstLine.match(/,/g)?.length ?? 0) ? ";" : ",";
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
  return rows.filter(r => r.some(c => c.trim()));
}

/** "05/11/2026", "5/11/26", "05-11-2026" (day first) or "2026-11-05" → "2026-11-05"; impossible dates → null. */
export function parseDate(text: string): string | null {
  const t = text.trim();
  const dmy = t.match(/^(\d{1,2})[/-](\d{1,2})[/-](\d{2}|\d{4})$/), iso = t.match(/^(\d{4})-(\d{1,2})-(\d{1,2})$/);
  const [y, m, d] = dmy ? [+dmy[3] < 100 ? 2000 + +dmy[3] : +dmy[3], +dmy[2], +dmy[1]] : iso ? [+iso[1], +iso[2], +iso[3]] : [0, 0, 0];
  if (!y) return null;
  const date = new Date(y, m - 1, d);
  if (date.getFullYear() !== y || date.getMonth() !== m - 1 || date.getDate() !== d) return null;
  return `${y}-${String(m).padStart(2, "0")}-${String(d).padStart(2, "0")}`;
}

const money = (text: string | undefined) => text === undefined || !text.trim() ? null : parseCents(text.replace(/\s/g, "").replace(/^\$-|^-\$/, "-"));

export function mapRows(rows: string[][], ctx: QuickContext): ImportRow[] {
  if (!rows.length) return [];
  const head = rows[0].map(h => norm(h.trim()));
  const hasHead = head.some(h => /fecha|date|monto|importe|amount|comercio|descripcion|concepto|description/.test(h));
  const col = (re: RegExp, fallback: number) => { const i = hasHead ? head.findIndex(h => re.test(h)) : -1; return i >= 0 ? i : fallback; };
  const iDate = col(/fecha|date/, 0), iPayee = col(/comercio|descripcion|concepto|description|payee|beneficiario/, 1);
  const iOut = col(/cargo|retiro|debito|debit|withdrawal/, -1), iIn = col(/abono|deposito|credito|credit/, -1);
  const iAmount = col(/monto|importe|cantidad|amount/, iOut >= 0 || iIn >= 0 ? -1 : 2), iCategory = col(/categoria|category/, -1);
  return rows.slice(hasHead ? 1 : 0).map((r, n) => {
    const date = parseDate(r[iDate] ?? ""), payee = (r[iPayee] ?? "").trim();
    let cents = iAmount >= 0 ? money(r[iAmount]) : null;
    if (iAmount < 0) {
      const out = money(r[iOut]), into = money(r[iIn]);
      cents = out || into ? (into ? Math.abs(into) : 0) - (out ? Math.abs(out) : 0) : null;
    }
    const named = iCategory >= 0 ? ctx.categories.find(c => norm(c.name) === norm((r[iCategory] ?? "").trim())) : undefined;
    const categoryId = named?.id ?? (payee ? parseQuick(payee, ctx).categoryId : null);
    const error = !date ? "Fecha no válida" : !cents ? "Monto no válido" : !payee ? "Sin comercio" : "";
    return { line: n + (hasHead ? 2 : 1), date, payee, cents, categoryId, error };
  });
}

/** Same account, day, amount and payee as an existing movement. */
export const isDuplicate = (row: ImportRow, existing: { date: string; amount: number; payeeName: string | null }[]) =>
  existing.some(e => e.date === row.date && e.amount === row.cents && norm(e.payeeName ?? "") === norm(row.payee));
