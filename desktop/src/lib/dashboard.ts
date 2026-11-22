// Which Resumen widgets show, in what order and how wide. Saved per profile in localStorage as [{ id, hidden, span }].

export interface WidgetDef { id: string; title: string; span: number }
/** `span` is the width in columns of a 6-column grid, 2 to 6. */
export interface Slot { id: string; hidden: boolean; span: number }

export const WIDGETS: WidgetDef[] = [
  { id: "hero", title: "Te sobra / te falta", span: 4 },
  { id: "pagos", title: "Pagos por hacer", span: 2 },
  { id: "kpis", title: "Indicadores", span: 6 },
  { id: "gasto", title: "¿En qué se va el dinero?", span: 3 },
  { id: "chart", title: "Ingresos vs gastos", span: 3 },
  { id: "presupuesto", title: "Presupuesto", span: 3 },
  { id: "recientes", title: "Últimos movimientos", span: 3 },
  { id: "cuentas", title: "Saldo por cuenta", span: 3 },
  { id: "deseos", title: "Deseos", span: 3 },
];

const MIN = 2, MAX = 6;

/** A saved layout, cleaned up: unknown ids dropped, widgets added in later versions appended visible. */
export function readLayout(saved: string | null): Slot[] {
  let list: Slot[] = [];
  try { const v = JSON.parse(saved ?? "[]"); if (Array.isArray(v)) list = v; } catch { /* default */ }
  const known = new Set(WIDGETS.map(w => w.id));
  const span = (id: string, saved?: unknown) => Number.isInteger(saved) && (saved as number) >= MIN && (saved as number) <= MAX ? saved as number : WIDGETS.find(w => w.id === id)!.span;
  const kept = list.filter((s, i) => s && known.has(s.id) && list.findIndex(x => x?.id === s.id) === i).map(s => ({ id: s.id, hidden: !!s.hidden, span: span(s.id, s.span) }));
  return [...kept, ...WIDGETS.filter(w => !kept.some(s => s.id === w.id)).map(w => ({ id: w.id, hidden: false, span: w.span }))];
}

/** Puts `id` right before or after `target` (at the end when `target` is null). Returns the same layout when nothing moves. */
export function placeAt(layout: Slot[], id: string, target: string | null, after: boolean): Slot[] {
  const moving = layout.find(s => s.id === id);
  if (!moving || target === id) return layout;
  const rest = layout.filter(s => s.id !== id);
  const at = target === null ? rest.length : rest.findIndex(s => s.id === target) + (after ? 1 : 0);
  if (at < 0 || (target !== null && !rest.some(s => s.id === target))) return layout;
  const next = [...rest.slice(0, at), moving, ...rest.slice(at)];
  return next.every((s, i) => s === layout[i]) ? layout : next;
}

export const toggle = (layout: Slot[], id: string): Slot[] => layout.map(s => s.id === id ? { ...s, hidden: !s.hidden } : s);

export const resize = (layout: Slot[], id: string, span: number): Slot[] =>
  layout.map(s => s.id === id ? { ...s, span: Math.max(MIN, Math.min(MAX, Math.round(span))) } : s);
