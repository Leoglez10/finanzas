import { invoke } from "@tauri-apps/api/core";

export interface AccountOverview { id: number; name: string; offBudget: boolean; closed: boolean; sortOrder: number; balance: number }
export interface EntryRow { id: number; date: string; amount: number; notes: string; cleared: boolean; accountId: number | null; categoryId: number | null; payeeId: number | null; accountName: string | null; payeeName: string | null; categoryName: string | null }
export interface EntryInput { date: string; amount: number; accountId: number; payee: string; categoryId: number | null; notes: string }
export interface Category { id: number; name: string; hidden: boolean }
export interface CategoryGroup { id: number; name: string; isIncome: boolean; categories: Category[] }
export interface Progress { categoryId: number; name: string; spent: number; budgeted: number }
export interface Overview { totalBalance: number; openAccountCount: number; current: {income: number; expenses: number; budgeted: number; savingsRate: number | null}; previous: {income: number; expenses: number}; incomeChange: number | null; expensesChange: number | null; chart: {month: number; income: number; expenses: number}[]; spendingTotal: number; spending: Progress[]; recentEntries: EntryRow[]; budgetSpent: number; budgetAssigned: number; budgetProgress: Progress[]; firstRun: {hasAccount: boolean; hasCategorizedIncome: boolean; hasBudget: boolean} }
export interface EntriesResult { rows: EntryRow[]; total: number; sum: number }
export interface BudgetRow { id: number; name: string; hidden: boolean; budgeted: number; activity: number; available: number; spentShare: number }
export interface BudgetView { toBudget: number; income: number; assigned: number; hint: string; uncategorizedIncome: number; groups: {id:number; name:string; isIncome:boolean; categories:BudgetRow[]}[] }
export interface ScheduleView { schedule: {id:number; name:string; amount:number; autoPost:boolean; completed:boolean}; nextDate:string|null; label:string; payee:string|null; account:string|null }
export interface UpcomingItem { scheduleId:number; name:string; date:string; amount:number; account:string|null; overdue:boolean; projectedBalance:number|null }
export interface Settings { enabled:boolean; days:number }
export interface ImportSummary { accounts:number; categories:number; payees:number; entries:number; schedules:number }
export const budget = (month:number) => invoke<BudgetView>('budget', {month});
export const assignBudget = (month:number, categoryId:number, text:string) => invoke<number|null>('assign_budget', {month, categoryId, text});
export const createAccount = (name:string) => invoke<void>('create_account', {name});
export const changeCategory = (id:number, name:string|null, hidden:boolean|null) => invoke<void>('change_category', {id, name, hidden});
export const changeGroup = (id:number|null, name:string) => invoke<void>('change_group', {id, name});
export const createCategory = (groupId:number, name:string) => invoke<void>('create_category', {groupId, name});
export const upcoming = () => invoke<UpcomingItem[]>('upcoming');
export const schedules = () => invoke<ScheduleView[]>('schedules');
export const createSchedule = (input: unknown) => invoke<void>('create_schedule', {input});
export const scheduleAction = (id:number, action:string) => invoke<void>('schedule_action', {id, action});
export const getSettings = () => invoke<Settings>('get_settings');
export const saveSettings = (enabled:boolean, days:number) => invoke<void>('save_settings', {enabled, days});
export const importAvailability = () => invoke<[boolean, boolean]>('import_availability');
export const importData = (source:string) => invoke<ImportSummary|null>('import_data', {source});
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
