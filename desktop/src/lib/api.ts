// Typed wrappers over the Rust commands in src-tauri. Money is always integer cents.
import { invoke } from "@tauri-apps/api/core";

export interface AccountOverview { id: number; name: string; offBudget: boolean; closed: boolean; sortOrder: number; balance: number }
export interface EntryRow { id: number; date: string; amount: number; notes: string; cleared: boolean; accountId: number | null; categoryId: number | null; payeeId: number | null; accountName: string | null; payeeName: string | null; categoryName: string | null }
export interface EntryInput { date: string; amount: number; accountId: number; payee: string; categoryId: number | null; notes: string }
export interface Category { id: number; name: string; hidden: boolean }
export interface CategoryGroup { id: number; name: string; isIncome: boolean; categories: Category[] }
export interface Progress { categoryId: number; name: string; spent: number; budgeted: number }
export interface MonthSummary { income: number; expenses: number; budgeted: number; savingsRate: number | null }
export interface Overview {
  totalBalance: number; openAccountCount: number; current: MonthSummary; previous: MonthSummary;
  incomeChange: number | null; expensesChange: number | null; chart: { month: number; income: number; expenses: number }[];
  spendingTotal: number; spending: Progress[]; recentEntries: EntryRow[]; budgetSpent: number; budgetAssigned: number; budgetProgress: Progress[];
  firstRun: { hasAccount: boolean; hasCategorizedIncome: boolean; hasBudget: boolean };
}
export interface EntriesResult { rows: EntryRow[]; total: number; sum: number }
export interface UpcomingItem { scheduleId: number; name: string; date: string; amount: number; accountId: number | null; account: string | null; autoPost: boolean; overdue: boolean; projectedBalance: number | null }
export interface Wish { id: number; name: string; price: number; url: string; notes: string; categoryId: number | null; reserved: number; archived: boolean }
export interface WishEvaluation { status: string; availableForWish: number | null; budgetShortfall: number | null; reservationDeficit: number; cashBalance: number | null; upcomingOutflows: number; cashShortfall: number | null; reasons: string[] }
export interface WishView { wish: Wish; evaluation: WishEvaluation }
export interface WishDraft { name: string; price: number; url: string; notes: string; categoryId: number | null }
export interface BudgetCategory { id: number; name: string; hidden: boolean; budgeted: number; activity: number; available: number; spentShare: number }
export interface BudgetView { month: number; toBudget: number; income: number; assigned: number; hint: string; uncategorizedIncome: number; groups: { id: number; name: string; isIncome: boolean; categories: BudgetCategory[] }[] }
export type Frequency = "daily" | "weekly" | "monthly" | "yearly";
export interface Recurrence { start: string; frequency: Frequency; interval: number; skipWeekend: boolean; weekendMode: string; endDate: string | null }
export interface Schedule { id: number; name: string; amount: number; autoPost: boolean; completed: boolean; nextDate: string | null; recurrence: Recurrence | null; accountId: number | null; categoryId: number | null; payeeId: number | null }
export interface ScheduleView { schedule: Schedule; nextDate: string | null; label: string; payee: string | null; account: string | null }
export interface ScheduleDraft { name: string; amount: number; accountId: number; categoryId: number | null; payee: string; frequency: Frequency; interval: number; start: string; skipWeekend: boolean; autoPost: boolean }
export interface Settings { enabled: boolean; days: number; hour: number; hidden_profiles: string[]; dark: boolean | null; palette: string; tray: boolean; autostart: boolean; auto_update: boolean; profile: string; profile_colors: Record<string, string> }
export interface ProfileView { slug: string; label: string; hidden: boolean; color: string | null }
export interface Profiles { active: string; list: ProfileView[] }
export interface ImportSummary { accounts: number; categories: number; payees: number; entries: number; schedules: number }

export const startupError = () => invoke<string | null>("startup_error");
export const accountsOverview = () => invoke<AccountOverview[]>("accounts_overview");
export const dataVersion = () => invoke<string>("data_version");
export const overview = (month: number, chartMonths: number) => invoke<Overview>("overview", { month, chartMonths });
export const entries = (filter: { accountId?: number; search?: string; limit?: number }) => invoke<EntriesResult>("entries", { filter });
export const addEntry = (entry: EntryInput) => invoke<EntryRow>("add_entry", { entry });
export const updateEntry = (id: number, entry: EntryInput) => invoke<EntryRow>("update_entry", { id, entry });
export const deleteEntry = (id: number) => invoke<EntryRow>("delete_entry", { id });
export const payeeSuggestions = (prefix: string) => invoke<string[]>("payee_suggestions", { prefix });
export const suggestedCategory = (payeeName: string) => invoke<number | null>("suggested_category", { payeeName });
/** The input that recreates a row exactly: used to undo deletes and edits. */
export const inputOf = (r: EntryRow): EntryInput => ({ date: r.date, amount: r.amount, accountId: r.accountId ?? 0, payee: r.payeeName ?? "", categoryId: r.categoryId, notes: r.notes });
export const categoriesGrouped = () => invoke<CategoryGroup[]>("categories_grouped");
export const budget = (month: number) => invoke<BudgetView>("budget", { month });
/** Accepts "800", "+100", "-50" or arithmetic like "550*2"; returns the new cents or null when the text is invalid. */
export const assignBudget = (month: number, categoryId: number, text: string) => invoke<number | null>("assign_budget", { month, categoryId, text });
export const upcoming = () => invoke<UpcomingItem[]>("upcoming");
export const schedules = () => invoke<ScheduleView[]>("schedules");
export const saveSchedule = (id: number | null, draft: ScheduleDraft) => invoke<number>("save_schedule", { id, draft });
export const postSchedule = (id: number) => invoke<{ entry: EntryRow; before: Schedule }>("post_schedule", { id });
export const skipSchedule = (id: number) => invoke<Schedule>("skip_schedule", { id });
export const deleteSchedule = (id: number) => invoke<Schedule>("delete_schedule", { id });
export const restoreSchedule = (schedule: Schedule) => invoke<number>("restore_schedule", { schedule });
export interface CsvFile { name: string; text: string }
export const readCsv = (path: string) => invoke<CsvFile>("read_csv", { path });
export const pickCsv = () => invoke<CsvFile | null>("pick_csv");
/** All rows or none; returns the new ids so the import can be undone. */
export const importEntries = (entries: EntryInput[]) => invoke<number[]>("import_entries", { entries });
/** Returns the new account id; a non-zero `opening` becomes a "Saldo inicial" movement dated `date`. */
export const createAccount = (name: string, offBudget: boolean, opening: number | null, date: string) => invoke<number>("create_account", { name, offBudget, opening, date });
export interface AgentView { name: string; connected: boolean; instructions: string; config: string }
export const agents = () => invoke<AgentView[]>("agents");
export const databasePath = () => invoke<string>("database_path");
export const refreshTray = () => invoke<void>("refresh_tray");
export const wishes = (archived = false) => invoke<WishView[]>("wishes", { archived });
export const saveWish = (id: number | null, draft: WishDraft) => invoke<Wish>("save_wish", { id, draft });
/** `amount` is the new total set aside (0 releases it), not a delta. */
export const reserveWish = (id: number, amount: number) => invoke<Wish>("reserve_wish", { id, amount });
export const archiveWish = (id: number) => invoke<Wish>("archive_wish", { id });
export const restoreWish = (wish: Wish) => invoke<void>("restore_wish", { wish });
export const today = () => invoke<string>("today");
export const getSettings = () => invoke<Settings>("get_settings");
export const updateSettings = (patch: Partial<Settings>) => invoke<Settings>("update_settings", { patch });
export const profiles = () => invoke<Profiles>("profiles");
export const switchProfile = (slug: string) => invoke<void>("switch_profile", { slug });
export const createProfile = (name: string) => invoke<string>("create_profile", { name });
export const renameProfile = (from: string, name: string) => invoke<string>("rename_profile", { from, name });
export const deleteProfile = (slug: string) => invoke<void>("delete_profile", { slug });
export const testNotification = () => invoke<void>("test_notification");
/** [store is empty, the old macOS app's data exists] */
export const importAvailability = () => invoke<[boolean, boolean]>("import_availability");
export const importData = (source: "actual" | "legacy") => invoke<ImportSummary | null>("import_data", { source });

// finanzas-core reports wish rules in English; show them in Spanish.
const MESSAGES: Record<string, string> = {
  "Not enough unreserved category funds": "No hay suficiente dinero libre en esa categoría del presupuesto.",
  "Choose an expense category before reserving": "Elige una categoría de gasto antes de apartar dinero.",
  "Reservation must be between zero and the wish price": "Lo apartado debe estar entre cero y el precio.",
  "Release reserved funds before reducing the price": "Libera lo apartado antes de bajar el precio.",
  "Wish link must use http:// or https://": "El enlace debe empezar con http:// o https://",
  "Wish name must not be empty": "Ponle un nombre.",
  "Wish price must be greater than zero": "El precio debe ser mayor a cero.",
  "Archived wishes cannot be edited": "Un deseo archivado no se puede editar.",
  "Income categories cannot be budgeted": "Elige una categoría de gasto, no de ingreso.",
};
export const errorText = (e: unknown) => MESSAGES[String(e)] ?? String(e);
