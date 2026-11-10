import { invoke } from "@tauri-apps/api/core";

export interface AccountOverview {
  id: number;
  name: string;
  offBudget: boolean;
  closed: boolean;
  sortOrder: number;
  balance: number;
}

export const startupError = () => invoke<string | null>("startup_error");
export const accountsOverview = () => invoke<AccountOverview[]>("accounts_overview");
export const dataVersion = () => invoke<number>("data_version");
