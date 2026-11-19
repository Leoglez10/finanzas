// App-wide state: route, accounts, settings and the data version every screen reloads on.
import * as api from "./api";

export const app = $state({
  route: location.hash.slice(1) || "resumen",
  version: "",
  error: null as string | null,
  accounts: [] as api.AccountOverview[],
  overdue: 0,
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
  const [accounts, upcoming] = await Promise.all([api.accountsOverview(), api.upcoming()]);
  app.accounts = accounts; app.overdue = upcoming.filter(u => u.overdue).length;
}
async function tick() {
  try {
    const version = await api.dataVersion();
    if (version !== app.version) { await refreshAccounts(); app.version = version; void api.refreshTray(); }
  } catch (e) { console.error(e); }
}
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
}
