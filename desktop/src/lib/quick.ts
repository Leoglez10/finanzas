// Shared by quick capture and the palette: parse free text against the user's own data and save it.
import * as api from "./api";
import { isoDay, money } from "./format";
import { parseQuick, type QuickContext, type QuickDraft } from "./quick-parse";

/** Recent movements decide payee spelling, category and account (most recent first). */
export async function quickContext(today: Date): Promise<QuickContext> {
  const [entries, groups, accounts] = await Promise.all([api.entries({ limit: 400 }), api.categoriesGrouped(), api.accountsOverview()]);
  return {
    today,
    history: entries.rows.filter(r => r.payeeName).map(r => ({ payee: r.payeeName!, categoryId: r.categoryId, accountId: r.accountId })),
    accounts: accounts.filter(a => !a.closed).map(a => ({ id: a.id, name: a.name })),
    categories: groups.flatMap(g => g.categories),
    incomeCategoryIds: groups.filter(g => g.isIncome).flatMap(g => g.categories.map(c => c.id)),
  };
}

export const parse = (text: string, ctx: QuickContext) => parseQuick(text, ctx);
export const signed = (d: QuickDraft) => d.kind === "gasto" ? -(d.cents ?? 0) : (d.cents ?? 0);
export const describe = (d: QuickDraft) => `${d.payee || "Sin comercio"} ${money(signed(d), true)}`;

/** Saves a parsed draft; the account falls back to the first open one. Returns the created row. */
export async function saveDraft(d: QuickDraft, ctx: QuickContext) {
  const accountId = d.accountId ?? ctx.accounts[0]?.id;
  if (!accountId) throw new Error("Primero crea una cuenta");
  if (!d.cents) throw new Error("Agrega un monto, por ejemplo 120");
  return api.addEntry({ date: isoDay(d.date), amount: signed(d), accountId, payee: d.payee, categoryId: d.categoryId, notes: "" });
}
