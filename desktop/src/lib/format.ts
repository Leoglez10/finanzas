// Formatting shared by every screen. Amounts arrive as integer cents; dates as "YYYY-MM-DD".
const currency = new Intl.NumberFormat("es-MX", { style: "currency", currency: "MXN", currencyDisplay: "narrowSymbol" });

// `|| 0` turns -0 into 0, so an empty total never shows as "-$0.00".
export const money = (cents: number, sign = false) => (sign && cents > 0 ? "+" : "") + currency.format((cents || 0) / 100);
/** "$1,234.56" split so the cents can be drawn smaller. */
export const moneyParts = (cents: number) => { const [whole, decimals = "00"] = currency.format((cents || 0) / 100).split("."); return { whole, decimals }; };

/** Local date from "YYYY-MM-DD" (Date.parse would read it as UTC and shift the day). */
export const parseDay = (iso: string) => { const [y, m, d] = iso.split("-").map(Number); return new Date(y, m - 1, d); };
export const isoDay = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
export const daysBetween = (from: Date, to: Date) => Math.round((Date.UTC(to.getFullYear(), to.getMonth(), to.getDate()) - Date.UTC(from.getFullYear(), from.getMonth(), from.getDate())) / 864e5);

/** Month keys are YYYYMM integers, as in finanzas-core. */
export const monthKey = (d: Date) => d.getFullYear() * 100 + d.getMonth() + 1;
export const monthDate = (key: number) => new Date(Math.floor(key / 100), (key % 100) - 1, 1);
export const shiftMonth = (key: number, by: number) => { const d = monthDate(key); d.setMonth(d.getMonth() + by); return monthKey(d); };
export const monthTitle = (key: number) => { const s = monthDate(key).toLocaleDateString("es-MX", { month: "long", year: "numeric" }).replace(" de ", " "); return s[0].toUpperCase() + s.slice(1); };
export const monthShort = (key: number) => monthDate(key).toLocaleDateString("es-MX", { month: "short" }).replace(".", "");
export const dayNumber = (d: Date) => d.toLocaleDateString("es-MX", { day: "2-digit" });
export const dayMonth = (d: Date) => d.toLocaleDateString("es-MX", { month: "short" }).replace(".", "");

/** "+2.1% vs octubre" from a ratio change such as 0.021. */
export const change = (ratio: number | null, previousKey: number) =>
  ratio === null ? "" : `${ratio >= 0 ? "+" : "−"}${Math.abs(ratio * 100).toFixed(1)}% vs ${monthDate(previousKey).toLocaleDateString("es-MX", { month: "long" })}`;

// ponytail: color by category id modulo a fixed set; let users pick per category when someone asks.
const CATEGORY_COLORS = ["var(--c-vivienda)", "var(--c-alimentacion)", "var(--c-transporte)", "var(--c-ocio)", "var(--c-servicios)", "var(--c-salud)", "#EC4899", "#84CC16"];
export const categoryColor = (id: number | null) => id === null ? "var(--muted)" : CATEGORY_COLORS[id % CATEGORY_COLORS.length];
