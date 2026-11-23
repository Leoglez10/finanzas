// Which dialog is open and with what. Dialog components watch this and call showModal().
import type { EntryRow, ScheduleView, Wish, WishItem } from "./api";

export interface EntryDraft { cents?: number | null; payee?: string; categoryId?: number | null; accountId?: number | null; date?: Date }
export const entryDialog = $state({ open: false, kind: "gasto" as "gasto" | "ingreso", row: null as EntryRow | null, draft: null as EntryDraft | null });

export function openEntry(kind: "gasto" | "ingreso", row: EntryRow | null = null, draft: EntryDraft | null = null) {
  Object.assign(entryDialog, { open: true, kind, row, draft });
}

export const scheduleDialog = $state({ open: false, view: null as ScheduleView | null });
export function openSchedule(view: ScheduleView | null = null) { Object.assign(scheduleDialog, { open: true, view }); }
export const wishDialog = $state({ open: false, wish: null as Wish | null, items: [] as WishItem[] });
export function openWish(wish: Wish | null = null, items: WishItem[] = []) { Object.assign(wishDialog, { open: true, wish, items }); }
import type { CsvFile } from "./api";
export const importDialog = $state({ open: false, file: null as CsvFile | null });
export function openImport(file: CsvFile) { Object.assign(importDialog, { open: true, file }); }
export const accountDialog = $state({ open: false });
export function openAccount() { accountDialog.open = true; }
import type { DebtView, Installment } from "./api";
export const cardDialog = $state({ open: false, debt: null as DebtView | null });
/** Without a debt it turns an existing account into a card. */
export function openCard(debt: DebtView | null = null) { Object.assign(cardDialog, { open: true, debt }); }
export const installmentDialog = $state({ open: false, accountId: 0, plan: null as Installment | null });
export function openInstallment(accountId: number, plan: Installment | null = null) { Object.assign(installmentDialog, { open: true, accountId, plan }); }
