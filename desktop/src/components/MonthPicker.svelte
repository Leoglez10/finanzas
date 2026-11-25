<script lang="ts" module>
  let next = 0;
</script>

<script lang="ts">
  // A button that opens month presets plus a 12-month grid: click a month to start, click it again
  // for just that month or another one to make a range. Month keys are YYYYMM.
  import { monthKey, monthShort, monthSpan, monthTitle, shiftMonth } from "../lib/format";
  import Icon from "./Icon.svelte";

  let { from, to, today, onchange }: { from: number; to: number; today: Date; onchange: (from: number, to: number) => void } = $props();

  const id = `months-${next++}`;
  let button: HTMLButtonElement;
  let pop: HTMLDivElement;
  let year = $state(0);
  let start = $state<number | null>(null);
  let hover = $state<number | null>(null);

  let now = $derived(monthKey(today));
  let presets = $derived<[string, number, number][]>([
    ["Este mes", now, now],
    ["Mes pasado", shiftMonth(now, -1), shiftMonth(now, -1)],
    ["Últimos 3 meses", shiftMonth(now, -2), now],
    ["Últimos 6 meses", shiftMonth(now, -5), now],
    ["Este año", Math.floor(now / 100) * 100 + 1, now],
    ["Año pasado", (Math.floor(now / 100) - 1) * 100 + 1, (Math.floor(now / 100) - 1) * 100 + 12],
  ]);
  let current = $derived(presets.find(([, f, t]) => f === from && t === to)?.[0]);
  const withYear = (k: number) => `${monthShort(k)} ${Math.floor(k / 100)}`;
  let label = $derived(current ?? monthSpan(from, to));
  let band = $derived(start ? (hover && hover < start ? [hover, start] : [start, hover ?? start]) : [from, to]);

  function toggled(e: ToggleEvent) {
    if (e.newState !== "open") { start = null; hover = null; return; }
    const r = button.getBoundingClientRect();
    pop.style.top = `${r.bottom + 6}px`;
    pop.style.left = `${Math.max(8, Math.min(r.left, innerWidth - pop.offsetWidth - 8))}px`;
    year = Math.floor(to / 100);
  }
  function choose(f: number, t: number) { onchange(f, t); pop.hidePopover(); }
  function pick(k: number) {
    if (!start) { start = k; return; }
    choose(Math.min(start, k), Math.max(start, k));
  }
</script>

<button bind:this={button} class="btn month-btn" popovertarget={id} aria-haspopup="dialog"><Icon name="cal" />{label}</button>
<div bind:this={pop} {id} popover="auto" class="month-pop" role="dialog" aria-label="Elegir periodo" ontoggle={toggled}>
  <div class="presets">
    {#each presets as [text, f, t] (text)}
      <button aria-pressed={current === text} onclick={() => choose(f, t)}>{text}</button>
    {/each}
  </div>
  <div class="cal">
    <div class="cal-h">
      <button class="btn icon sm ghost" aria-label="Año anterior" onclick={() => year--}><Icon name="left" /></button>
      <b class="num">{year}</b>
      <button class="btn icon sm ghost" aria-label="Año siguiente" onclick={() => year++}><Icon name="right" /></button>
    </div>
    <div class="grid3">
      {#each Array.from({ length: 12 }, (_, i) => year * 100 + i + 1) as k (k)}
        <button class="m" class:now={k === now} class:in={k >= band[0] && k <= band[1]} class:edge={k === band[0] || k === band[1]}
          aria-label={monthTitle(k)} onclick={() => pick(k)} onpointerenter={() => hover = k}>{monthShort(k)}</button>
      {/each}
    </div>
    <p class="hint">{start ? `Desde ${withYear(start)} · elige el último mes (o el mismo)` : "Elige el primer mes"}</p>
  </div>
</div>

<style>
  .month-btn { font-weight: 550; }
  .month-pop { position: fixed; inset: auto; margin: 0; padding: 12px; gap: 12px; border: 1px solid var(--border); border-radius: var(--r-panel); background: var(--surface); color: inherit; box-shadow: var(--shadow-float); }
  .month-pop:popover-open { display: flex; }
  .presets { display: flex; flex-direction: column; gap: 2px; min-width: 150px; padding-right: 12px; border-right: 1px solid var(--border); }
  .presets button { text-align: left; border: 0; background: none; padding: 8px 10px; border-radius: var(--r-control); cursor: pointer; font-size: var(--text-md); font-weight: 550; color: var(--muted); }
  .presets button:hover { background: var(--surface-2); color: inherit; }
  .presets button[aria-pressed="true"] { background: var(--brand-soft); color: var(--brand); }
  .cal { width: 240px; }
  .cal-h { display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px; }
  .grid3 { display: grid; grid-template-columns: repeat(3, 1fr); gap: 4px 0; }
  .m { height: 40px; border: 0; background: none; cursor: pointer; font-size: var(--text-md); text-transform: capitalize; border-radius: var(--r-control); }
  .m:hover { background: var(--surface-2); }
  .m.now { font-weight: 700; color: var(--brand); }
  .m.in { background: var(--brand-soft); border-radius: 0; }
  .m.edge { background: var(--brand); color: var(--brand-ink); border-radius: var(--r-control); }
  .hint { margin: 8px 0 0; font-size: var(--text-sm); color: var(--muted); text-align: center; }
</style>
