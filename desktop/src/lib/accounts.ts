// Account look: preset icons and colors (same names finanzas-core accepts) and the default for each account.
export const ACCOUNT_ICONS: [string, string][] = [["bank", "Banco"], ["card", "Tarjeta"], ["cash", "Efectivo"], ["wallet", "Cartera"], ["piggy", "Ahorro"], ["chart", "Inversión"], ["house", "Casa"], ["car", "Auto"], ["gift", "Regalo"], ["shield", "Emergencias"], ["star", "Favorita"], ["plane", "Viajes"]];
export const ACCOUNT_COLORS: [string, string, string][] = [["violet", "Violeta", "#7C5CF5"], ["blue", "Azul", "#3B82F6"], ["teal", "Turquesa", "#14B8A6"], ["green", "Verde", "#22A06B"], ["lime", "Lima", "#84CC16"], ["yellow", "Amarillo", "#EAB308"], ["orange", "Naranja", "#F97316"], ["red", "Rojo", "#EF4444"], ["pink", "Rosa", "#EC4899"], ["gray", "Gris", "#64748B"]];

export interface AccountLook { offBudget: boolean; icon: string | null; color: string | null; image: string | null }
/** Accounts without a chosen icon keep the old look: a chart for off-budget, a card otherwise. */
export const accountIcon = (a: Pick<AccountLook, "offBudget" | "icon">) => a.icon ?? (a.offBudget ? "chart" : "card");
export const accountColor = (name: string | null) => ACCOUNT_COLORS.find(c => c[0] === name)?.[2] ?? null;
