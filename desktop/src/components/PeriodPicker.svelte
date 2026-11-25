<script lang="ts" module>
  let next = 0;
</script>

<script lang="ts">
  // A button that opens presets plus a month calendar: the first click on a day starts the range, the second ends it.
  import { untrack } from "svelte";
  import { PRESETS, presetRange, presetOf, monthGrid, type Range } from "../lib/period";
  import { isoDay, parseDay } from "../lib/format";
  import Icon from "./Icon.svelte";

  let { range, today, onchange }: { range: Range; today: Date; onchange: (r: Range) => void } = $props();

  const id = `period-${next++}`;
  let button: HTMLButtonElement;
  let pop: HTMLDivElement;
  let view = $state(untrack(() => new Date(today.getFullYear(), today.getMonth(), 1)));
  let start = $state<string | null>(null);
  let hover = $state<string | null>(null);

  const short = (iso: string, year = false) => parseDay(iso).toLocaleDateString("es-MX", { day: "numeric", month: "short", year: year ? "numeric" : undefined }).replace(".", "");
  let preset = $derived(presetOf(range, today));
  let label = $derived(preset !== "rango" ? PRESETS.find(([p]) => p === preset)![1]
    : range.from && range.to ? `${short(range.from)} – ${short(range.to, true)}`
    : range.from ? `Desde ${short(range.from, true)}` : `Hasta ${short(range.to, true)}`);
  let days = $derived(monthGrid(view.getFullYear(), view.getMonth()));
  let todayIso = $derived(isoDay(today));
  // While picking, the band follows the pointer from the first day; otherwise it shows the current range.
  let band = $derived(start ? (hover && hover < start ? { from: hover, to: start } : { from: start, to: hover ?? start }) : range);

  function toggled(e: ToggleEvent) {
    if (e.newState !== "open") { start = null; hover = null; return; }
    const r = button.getBoundingClientRect();
    pop.style.top = `${r.bottom + 6}px`;
    pop.style.left = `${Math.max(8, Math.min(r.left, innerWidth - pop.offsetWidth - 8))}px`;
    const anchor = range.to || range.from;
    if (anchor) { const d = parseDay(anchor); view = new Date(d.getFullYear(), d.getMonth(), 1); }
  }
  function choose(r: Range) { onchange(r); pop.hidePopover(); }
  function pick(iso: string) {
    if (!start) { start = iso; return; }
    choose(iso < start ? { from: iso, to: start } : { from: start, to: iso });
  }
  const shift = (by: number) => view = new Date(view.getFullYear(), view.getMonth() + by, 1);
</script>

<button bind:this={button} class="btn period-btn" popovertarget={id} aria-haspopup="dialog"><Icon name="cal" />{label}</button>
<div bind:this={pop} {id} popover="auto" class="period-pop" role="dialog" aria-label="Elegir periodo" ontoggle={toggled}>
  <div class="presets">
    {#each PRESETS as [value, text] (value)}
      <button aria-pressed={preset === value} onclick={() => choose(presetRange(value, today))}>{text}</button>
    {/each}
  </div>
  <div class="cal">
    <div class="cal-h">
      <button class="btn icon sm ghost" aria-label="Mes anterior" onclick={() => shift(-1)}><Icon name="left" /></button>
      <b>{view.toLocaleDateString("es-MX", { month: "long", year: "numeric" })}</b>
      <button class="btn icon sm ghost" aria-label="Mes siguiente" onclick={() => shift(1)}><Icon name="right" /></button>
    </div>
    <div class="grid7" role="grid" aria-label="Calendario">
      {#each ["L", "M", "M", "J", "V", "S", "D"] as d, i (i)}<span class="wd" aria-hidden="true">{d}</span>{/each}
      {#each days as iso (iso)}
        {@const d = parseDay(iso)}
        <button class="day" class:out={d.getMonth() !== view.getMonth()} class:today={iso === todayIso}
          class:in={(!band.from || iso >= band.from) && (!band.to || iso <= band.to) && (band.from || band.to)}
          class:edge={iso === band.from || iso === band.to}
          aria-label={parseDay(iso).toLocaleDateString("es-MX", { day: "numeric", month: "long", year: "numeric" })}
          onclick={() => pick(iso)} onpointerenter={() => hover = iso}>{d.getDate()}</button>
      {/each}
    </div>
    <p class="hint">{start ? `Desde ${short(start, true)} · elige el último día` : "Elige el primer día del rango"}</p>
  </div>
</div>

<style>
  .period-btn { font-weight: 550; }
  .period-pop { position: fixed; inset: auto; margin: 0; padding: 12px; gap: 12px; border: 1px solid var(--border); border-radius: var(--r-panel); background: var(--surface); color: inherit; box-shadow: var(--shadow-float); }
  .period-pop:popover-open { display: flex; }
  .presets { display: flex; flex-direction: column; gap: 2px; min-width: 150px; padding-right: 12px; border-right: 1px solid var(--border); }
  .presets button { text-align: left; border: 0; background: none; padding: 8px 10px; border-radius: var(--r-control); cursor: pointer; font-size: var(--text-md); font-weight: 550; color: var(--muted); }
  .presets button:hover { background: var(--surface-2); color: inherit; }
  .presets button[aria-pressed="true"] { background: var(--brand-soft); color: var(--brand); }
  .cal { width: 252px; }
  .cal-h { display: flex; align-items: center; justify-content: space-between; margin-bottom: 6px; }
  .cal-h b { text-transform: capitalize; font-size: var(--text-md); }
  .grid7 { display: grid; grid-template-columns: repeat(7, 1fr); }
  .wd { text-align: center; font-size: var(--text-sm); color: var(--muted); padding: 4px 0; }
  .day { aspect-ratio: 1; border: 0; background: none; cursor: pointer; font-size: var(--text-md); font-variant-numeric: tabular-nums; border-radius: var(--r-control); }
  .day:hover { background: var(--surface-2); }
  .day.out { color: var(--muted); opacity: .55; }
  .day.today { font-weight: 700; color: var(--brand); }
  .day.in { background: var(--brand-soft); border-radius: 0; }
  .day.edge { background: var(--brand); color: var(--brand-ink); border-radius: var(--r-control); opacity: 1; }
  .hint { margin: 8px 0 0; font-size: var(--text-sm); color: var(--muted); text-align: center; }
</style>
