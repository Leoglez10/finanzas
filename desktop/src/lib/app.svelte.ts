// App-wide state: route, accounts, settings and the data version every screen reloads on.
import * as api from "./api";
import { checkUpdates } from "./updates.svelte";

export const app = $state({
  route: location.hash.slice(1) || "resumen",
  version: "",
  error: null as string | null,
  accounts: [] as api.AccountOverview[],
  cards: [] as api.DebtView[], // accounts set up as credit cards, so any screen can open their data
  overdue: 0,
  profiles: { active: "personal", list: [] } as api.Profiles,
  settings: null as api.Settings | null,
  today: new Date(),
});

addEventListener("hashchange", () => { app.route = location.hash.slice(1) || "resumen"; });
export const go = (route: string) => { location.hash = route; };

/** Applies light/dark and the color theme to the document; tokens in app.css do the rest. */
export function applyAppearance(settings: api.Settings) {
  const root = document.documentElement;
  if (settings.dark === null) delete root.dataset.theme; else root.dataset.theme = settings.dark ? "dark" : "light";
  root.dataset.palette = settings.palette;
}

async function refreshAccounts() {
  const [accounts, upcoming, profiles, debts] = await Promise.all([api.accountsOverview(), api.upcoming(), api.profiles(), api.debts()]);
  app.accounts = accounts; app.cards = debts.debts.filter(d => d.isCard); app.overdue = upcoming.filter(u => u.overdue).length; app.profiles = profiles;
}
async function tick() {
  try {
    const version = await api.dataVersion();
    if (version !== app.version) { await refreshAccounts(); app.version = version; void api.refreshTray(); }
  } catch (e) { console.error(e); }
}
/** Saves preference fields and applies the look right away. */
export async function updateSettings(patch: Partial<api.Settings>) {
  const settings = await api.updateSettings(patch);
  app.settings = settings; applyAppearance(settings);
  // Hidden profiles and colors live in settings, which do not bump the data version.
  app.profiles = await api.profiles();
}
export const PROFILE_COLORS = ["var(--violet)", "var(--c-vivienda)", "var(--c-transporte)", "var(--c-ocio)", "var(--c-alimentacion)", "var(--c-salud)"];
export const profileColor = (p: api.ProfileView, index: number) => p.color ?? PROFILE_COLORS[index % PROFILE_COLORS.length];
/** Call after a write so screens reload now instead of on the next poll. */
export const refresh = tick;

/** Polls the data version so writes from the MCP server or another window show up within ~2 s. */
export async function start() {
  app.error = await api.startupError();
  if (app.error) return;
  const [today, settings] = await Promise.all([api.today(), api.getSettings()]);
  const [y, m, d] = today.split("-").map(Number);
  app.today = new Date(y, m - 1, d);
  app.settings = settings; applyAppearance(settings);
  await tick();
  setInterval(tick, 2000);
  // Dev builds never call GitHub on their own; use Ajustes › Actualizaciones to try it.
  if (settings.auto_update && !import.meta.env.DEV) void checkUpdates(true);
}
