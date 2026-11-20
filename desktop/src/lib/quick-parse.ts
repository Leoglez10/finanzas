// Free-text movement parser for quick capture and the command palette: "uber 120 ayer" → a draft.
// Amounts are integer cents. Ported from design/redesign/quick-parse.js.

export const norm = (s: string) => s.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();
const MONEY = /^([+-])?\$?((\d{1,3}(,\d{3})+|\d+)(\.\d{1,2})?)$/;
const WEEKDAYS = ["domingo", "lunes", "martes", "miercoles", "jueves", "viernes", "sabado"];
const INCOME = ["ingreso", "cobre", "cobro", "sueldo", "nomina", "reembolso", "deposito"];
const FILLER = ["de", "en", "a", "para", "por", "con", "el", "la", "los", "las", "hoy"];
// ponytail: fixed keyword list; the user's own history wins first (below). Extend when misses show up.
const KEYWORDS: Record<string, string[]> = {
  transporte: ["uber", "didi", "gasolina", "gasolinera", "taxi", "metro", "camion", "estacionamiento"],
  alimentacion: ["super", "supermercado", "comida", "tacos", "restaurante", "cafe", "oxxo", "despensa"],
  salud: ["farmacia", "doctor", "medicina", "dentista"],
  servicios: ["cfe", "luz", "agua", "internet", "telefono", "gas"],
  ocio: ["netflix", "spotify", "cine", "juego", "concierto"],
  vivienda: ["renta", "casa", "mantenimiento"],
};

/** "1,250.50" → 125050 cents, "-80" → -8000, "1,2" → null. Comma thousands, period decimals only. */
export function parseCents(raw: string): number | null {
  const m = raw.trim().match(MONEY);
  if (!m) return null;
  const cents = Math.round(parseFloat(m[2].replace(/,/g, "")) * 100);
  return m[1] === "-" ? -cents : cents;
}

export interface HistoryItem { payee: string; categoryId: number | null; accountId: number | null }
export interface QuickDraft { kind: "gasto" | "ingreso"; cents: number | null; payee: string; categoryId: number | null; accountId: number | null; date: Date }
export interface QuickContext { today: Date; history: HistoryItem[]; accounts: { id: number; name: string }[]; categories: { id: number; name: string }[]; incomeCategoryIds: number[] }

const addDays = (d: Date, n: number) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);

/** history is most-recent-first; it decides payee spelling, category and account. */
export function parseQuick(text: string, ctx: QuickContext): QuickDraft {
  const out: QuickDraft = { kind: "gasto", cents: null, payee: "", categoryId: null, accountId: null, date: addDays(ctx.today, 0) };
  const words: string[] = [];
  for (const raw of text.trim().split(/\s+/).filter(Boolean)) {
    const w = norm(raw);
    const cents = out.cents === null ? parseCents(raw) : null;
    if (cents !== null) { out.cents = Math.abs(cents); if (raw.startsWith("+")) out.kind = "ingreso"; continue; }
    if (w === "ayer") { out.date = addDays(ctx.today, -1); continue; }
    if (w === "antier" || w === "anteayer") { out.date = addDays(ctx.today, -2); continue; }
    const wd = WEEKDAYS.indexOf(w);
    if (wd >= 0) { out.date = addDays(ctx.today, -((ctx.today.getDay() - wd + 7) % 7)); continue; }
    const dm = raw.match(/^(\d{1,2})\/(\d{1,2})(?:\/(\d{4}))?$/);
    if (dm) { out.date = new Date(dm[3] ? +dm[3] : ctx.today.getFullYear(), +dm[2] - 1, +dm[1]); continue; }
    if (INCOME.includes(w)) { out.kind = "ingreso"; if (w === "sueldo" || w === "nomina") words.push(raw); continue; }
    const account = ctx.accounts.find(a => norm(a.name) === w);
    if (account) { out.accountId = account.id; continue; }
    if (FILLER.includes(w)) continue;
    words.push(raw);
  }
  const p = norm(words.join(" "));
  const past = p ? (ctx.history.find(h => norm(h.payee) === p) ?? ctx.history.find(h => norm(h.payee).startsWith(p))) : undefined;
  if (past) {
    out.payee = past.payee; out.categoryId = past.categoryId;
    out.accountId ??= past.accountId;
  } else {
    out.payee = p ? words.join(" ").replace(/^./, c => c.toUpperCase()) : "";
    const tokens = p.split(" ");
    const key = Object.keys(KEYWORDS).find(k => KEYWORDS[k].some(word => tokens.includes(word)));
    out.categoryId = key ? ctx.categories.find(c => norm(c.name) === key)?.id ?? null : null;
  }
  if (out.categoryId !== null && ctx.incomeCategoryIds.includes(out.categoryId)) out.kind = "ingreso";
  return out;
}
