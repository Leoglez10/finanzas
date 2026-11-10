import { invoke } from "@tauri-apps/api/core";

export interface AccountOverview { id: number; name: string; offBudget: boolean; closed: boolean; sortOrder: number; balance: number }
export interface EntryRow { id: number; date: string; amount: number; notes: string; cleared: boolean; accountId: number | null; categoryId: number | null; payeeId: number | null; accountName: string | null; payeeName: string | null; categoryName: string | null }
export interface EntryInput { date: string; amount: number; accountId: number; payee: string; categoryId: number | null; notes: string }
export interface Category { id: number; name: string; hidden: boolean }
export interface CategoryGroup { id: number; name: string; isIncome: boolean; categories: Category[] }
export interface Progress { categoryId: number; name: string; spent: number; budgeted: number }
export interface Overview { totalBalance: number; openAccountCount: number; current: {income: number; expenses: number; budgeted: number; savingsRate: number | null}; previous: {income: number; expenses: number}; incomeChange: number | null; expensesChange: number | null; chart: {month: number; income: number; expenses: number}[]; spendingTotal: number; spending: Progress[]; recentEntries: EntryRow[]; budgetSpent: number; budgetAssigned: number; budgetProgress: Progress[]; firstRun: {hasAccount: boolean; hasCategorizedIncome: boolean; hasBudget: boolean} }
export interface EntriesResult { rows: EntryRow[]; total: number; sum: number }
export const startupError = () => invoke<string | null>("startup_error");
export const accountsOverview = () => invoke<AccountOverview[]>("accounts_overview");
export const dataVersion = () => invoke<string>("data_version");
export const overview = (month: number, chartMonths: number) => invoke<Overview>("overview", {month, chartMonths});
export const entries = (filter: {accountId?: number; search?: string; limit?: number}) => invoke<EntriesResult>("entries", {filter});
export const addEntry = (entry: EntryInput) => invoke<EntryRow>("add_entry", {entry});
export const updateEntry = (id: number, entry: EntryInput) => invoke<EntryRow>("update_entry", {id, entry});
export const deleteEntry = (id: number) => invoke<EntryRow>("delete_entry", {id});
export const payeeSuggestions = (prefix: string) => invoke<string[]>("payee_suggestions", {prefix});
export const suggestedCategory = (payeeName: string) => invoke<number | null>("suggested_category", {payeeName});
export const categoriesGrouped = () => invoke<CategoryGroup[]>("categories_grouped");
export const today = () => invoke<string>("today");
export const parseAmount = (text: string) => invoke<number | null>("parse_amount", {text});
export const formatMoney = (cents: number) => new Intl.NumberFormat("es-MX", {style: "currency", currency: "MXN"}).format(cents / 100);
export const formatDate = (date: string) => date.split("-").reverse().join("/");
