// Period filters shared by the ledger screens. Ranges are inclusive ISO days; "" means open-ended,
// so plain string comparison works against EntryRow.date.
import { isoDay, monthDate } from "./format.ts";

export interface Range { from: string; to: string }
export type Preset = "todo" | "semana" | "mes" | "mes-pasado" | "3m" | "anio";

export const PRESETS: [Preset, string][] = [
  ["todo", "Todo el tiempo"], ["semana", "Esta semana"], ["mes", "Este mes"],
  ["mes-pasado", "Mes pasado"], ["3m", "Últimos 3 meses"], ["anio", "Este año"],
];

const monthRange = (y: number, m: number, months = 1): Range =>
  ({ from: isoDay(new Date(y, m, 1)), to: isoDay(new Date(y, m + months, 0)) });

export function presetRange(preset: Preset, today: Date): Range {
  const y = today.getFullYear(), m = today.getMonth();
  switch (preset) {
    case "todo": return { from: "", to: "" };
    case "semana": {
      const monday = new Date(y, m, today.getDate() - (today.getDay() + 6) % 7);
      return { from: isoDay(monday), to: isoDay(new Date(monday.getFullYear(), monday.getMonth(), monday.getDate() + 6)) };
    }
    case "mes": return monthRange(y, m);
    case "mes-pasado": return monthRange(y, m - 1);
    case "3m": return monthRange(y, m - 2, 3);
    case "anio": return { from: `${y}-01-01`, to: `${y}-12-31` };
  }
}

/** The preset a range matches today, or "rango" for a hand-picked one. */
export const presetOf = (r: Range, today: Date): Preset | "rango" =>
  PRESETS.find(([p]) => { const x = presetRange(p, today); return x.from === r.from && x.to === r.to; })?.[0] ?? "rango";

/** Reads desde/hasta from a route; the older mes=YYYYMM (links from Resumen) still works. */
export function rangeOfParams(p: URLSearchParams): Range {
  const mes = Number(p.get("mes"));
  if (mes && !p.has("desde") && !p.has("hasta")) { const d = monthDate(mes); return monthRange(d.getFullYear(), d.getMonth()); }
  return { from: p.get("desde") ?? "", to: p.get("hasta") ?? "" };
}

export const inRange = (iso: string, r: Range) => (!r.from || iso >= r.from) && (!r.to || iso <= r.to);

/** 42 ISO days (six Monday-first weeks) for a month calendar; `month` is 0-based. */
export function monthGrid(year: number, month: number): string[] {
  const first = new Date(year, month, 1);
  const start = 1 - (first.getDay() + 6) % 7;
  return Array.from({ length: 42 }, (_, i) => isoDay(new Date(year, month, start + i)));
}
