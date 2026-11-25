// Credit-card form fields and the advice shown on the Deudas page. Pure, so it is tested without the app.
import type { CardTerms, DebtView } from "./api";
import { money, parseDay } from "./format.ts";
import { parseCents } from "./quick-parse.ts";

export interface CardForm { limit: string; cutDay: string; dueDay: string; rate: string; minPayment: string }
export const emptyCardForm = (): CardForm => ({ limit: "", cutDay: "", dueDay: "", rate: "", minPayment: "" });

/** Percent text ("45", "45.5", "45,5 %") → basis points; "" → null; invalid → undefined. */
export function parsePercent(raw: string): number | null | undefined {
  const t = raw.replace("%", "").replace(",", ".").trim();
  if (!t) return null;
  if (!/^\d+(\.\d{1,2})?$/.test(t)) return undefined;
  return Math.round(Number(t) * 100);
}
const percentText = (bp: number | null) => bp === null ? "" : String(bp / 100);

export const cardForm = (t: CardTerms | null): CardForm => ({
  limit: t?.creditLimit ? (t.creditLimit / 100).toFixed(2) : "", cutDay: t?.cutDay ? String(t.cutDay) : "", dueDay: t?.dueDay ? String(t.dueDay) : "",
  rate: percentText(t?.annualRate ?? null), minPayment: percentText(t?.minPayment ?? null),
});

/** One line with the card data that is set, for the Deudas card ("Tasa 45.5% · Corte día 12 · …"). */
export function termsLine(t: CardTerms | null): string {
  if (!t) return "";
  const parts = [
    t.annualRate !== null ? `Tasa ${t.annualRate / 100}%` : "",
    t.cutDay ? `Corte día ${t.cutDay}` : "",
    t.dueDay ? `Pago día ${t.dueDay}` : "",
    t.minPayment !== null ? `Mínimo ${t.minPayment / 100}%` : "",
    t.creditLimit ? `Límite ${money(t.creditLimit)}` : "",
  ];
  return parts.filter(Boolean).join(" · ");
}

/** Every field is optional: empty ones stay null. Returns an error text for the first invalid one. */
export function cardTerms(f: CardForm): CardTerms | string {
  const limit = f.limit.trim() ? parseCents(f.limit) : null;
  if (limit === undefined || (f.limit.trim() && (!limit || limit <= 0))) return "Escribe un límite mayor a cero, por ejemplo 25,000";
  const day = (s: string) => s.trim() ? (/^\d{1,2}$/.test(s.trim()) && +s >= 1 && +s <= 31 ? +s : undefined) : null;
  const cutDay = day(f.cutDay), dueDay = day(f.dueDay);
  if (cutDay === undefined || dueDay === undefined) return "Los días van del 1 al 31";
  const annualRate = parsePercent(f.rate), minPayment = parsePercent(f.minPayment);
  if (annualRate === undefined || (annualRate ?? 0) > 100_000) return "Escribe la tasa como porcentaje, por ejemplo 45.5";
  if (minPayment === undefined || (minPayment ?? 0) > 10_000) return "El pago mínimo es un porcentaje de 0 a 100";
  return { creditLimit: limit, cutDay, dueDay, annualRate, minPayment };
}

const MISSING: Record<string, string> = { creditLimit: "límite", cutDay: "día de corte", dueDay: "día de pago", annualRate: "tasa", minPayment: "pago mínimo" };
const list = (items: string[]) => items.length < 2 ? items.join("") : `${items.slice(0, -1).join(", ")} y ${items.at(-1)}`;
export const missingText = (d: DebtView) => list(d.missing.map(m => MISSING[m] ?? m));
const longDate = (iso: string) => parseDay(iso).toLocaleDateString("es-MX", { day: "numeric", month: "long" });

const CARD_NAME = /tarjeta|\btdc\b|cr[eé]dito|visa|master ?card|amex|american express/i;
/** Accounts that look like debts: worth offering the Deudas page to someone who has not turned it on. */
export const debtCandidates = (accounts: { name: string; balance: number; closed: boolean }[]) =>
  accounts.filter(a => !a.closed && (a.balance < 0 || CARD_NAME.test(a.name)));
export const looksLikeCard = (name: string) => CARD_NAME.test(name);

export interface Advice { tone: "bad" | "warn" | "ok" | "info"; text: string }
/** What to do, most urgent first. */
export function advice(debts: DebtView[]): Advice[] {
  const out: Advice[] = [];
  for (const d of debts.filter(d => d.overdue)) out.push({ tone: "bad", text: `El pago de ${d.name} venció el ${longDate(d.dueDate!)}: paga ${money(d.toAvoidInterest ?? 0)} cuanto antes para frenar intereses y recargos.` });
  for (const d of debts.filter(d => !d.overdue && (d.toAvoidInterest ?? 0) > 0)) {
    let text = `Paga ${money(d.toAvoidInterest!)} de ${d.name}${d.dueDate ? ` antes del ${longDate(d.dueDate)}` : ""} para no pagar intereses.`;
    if (d.minimum !== null && d.interestIfMinimum) text += ` Si pagas solo el mínimo (${money(d.minimum)}), te cobrarían unos ${money(d.interestIfMinimum)} de intereses.`;
    out.push({ tone: "warn", text });
  }
  for (const d of debts.filter(d => (d.utilization ?? 0) > 0.3)) out.push({ tone: "warn", text: `Usas el ${Math.round(d.utilization! * 100)}% del límite de ${d.name}. Mantenerlo abajo del 30% cuida tu historial crediticio.` });
  const rated = debts.filter(d => d.debt > 0 && d.terms?.annualRate != null);
  if (rated.length > 1) out.push({ tone: "ok", text: `Si te sobra dinero, abónalo primero a ${rated[0].name}: tiene la tasa más alta (${rated[0].terms!.annualRate! / 100}%).` });
  return out;
}
