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
export interface WishView { wish: Wish; evaluation: { status: string; budgetShortfall: number | null; cashShortfall: number | null; reasons: string[] } }
export interface Settings { enabled: boolean; days: number; hour: number; hidden_profiles: string[]; dark: boolean | null; palette: string; tray: boolean; autostart: boolean; auto_update: boolean }

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
export const upcoming = () => invoke<UpcomingItem[]>("upcoming");
export const wishes = () => invoke<WishView[]>("wishes");
export const today = () => invoke<string>("today");
export const getSettings = () => invoke<Settings>("get_settings");
