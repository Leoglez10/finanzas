// Goal types (a purchase, a trip, an experience or a savings fund) and how a goal's plan reads.
import { money, parseDay } from "./format.ts";

export type Kind = "compra" | "viaje" | "experiencia" | "fondo";
export interface Plan { targetDate: string; remaining: number; monthsLeft: number; monthly: number; overdue: boolean }
interface KindInfo { id: Kind; label: string; icon: string; namePlaceholder: string; pricePlaceholder: string; done: string }

export const KINDS: KindInfo[] = [
  { id: "compra", label: "Compra", icon: "gift", namePlaceholder: "PlayStation 5", pricePlaceholder: "11,000.00", done: "Lo compré" },
  { id: "viaje", label: "Viaje", icon: "plane", namePlaceholder: "Viaje a Cancún", pricePlaceholder: "25,000.00", done: "Ya viajé" },
  { id: "experiencia", label: "Experiencia", icon: "star", namePlaceholder: "Concierto, curso, cena…", pricePlaceholder: "3,500.00", done: "Lo viví" },
  { id: "fondo", label: "Fondo", icon: "shield", namePlaceholder: "Fondo de emergencia", pricePlaceholder: "50,000.00", done: "Meta cumplida" },
];
export const kindOf = (id: string | undefined) => KINDS.find(k => k.id === id) ?? KINDS[0];

const shortDate = (iso: string) => parseDay(iso).toLocaleDateString("es-MX", { day: "numeric", month: "short", year: "numeric" }).replace(".", "").replace(/ de /g, " ");

/** "Aparta $1,200.00 al mes · 7 meses, hasta el 15 jun 2027" */
export function planText(p: Plan): string {
  const until = shortDate(p.targetDate);
  if (p.remaining <= 0) return `Meta alcanzada antes del ${until}`;
  if (p.overdue) return `Vencida el ${until} · faltan ${money(p.remaining)}`;
  if (p.monthsLeft <= 1) return `Aparta ${money(p.monthly)} este mes · hasta el ${until}`;
  return `Aparta ${money(p.monthly)} al mes · ${p.monthsLeft} meses, hasta el ${until}`;
}
