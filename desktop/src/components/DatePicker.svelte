<script lang="ts" module>
  let next = 0;
</script>

<script lang="ts">
  // A field-styled button that opens a month calendar in a popover. `value` is an ISO day
  // (YYYY-MM-DD), or "" when `optional` lets the user clear it.
  import { tick } from "svelte";
  import { app } from "../lib/app.svelte";
  import { isoDay, parseDay } from "../lib/format";
  import Icon from "./Icon.svelte";

  let { value = $bindable(""), label, optional = false }: { value?: string; label: string; optional?: boolean } = $props();

  const id = `date-${next++}`;
  const WEEKDAYS = ["do", "lu", "ma", "mi", "ju", "vi", "sá"];
  let button: HTMLButtonElement;
  let pop: HTMLDivElement;
  let month = $state(new Date());

  let today = $derived(isoDay(app.today));
  let text = $derived(value ? parseDay(value).toLocaleDateString("es-MX", { weekday: "short", day: "numeric", month: "long", year: "numeric" }) : "Sin fecha");
  let title = $derived(month.toLocaleDateString("es-MX", { month: "long", year: "numeric" }).replace(" de ", " "));
  // Six weeks starting on the Sunday on or before the 1st, so the grid never changes height.
  let days = $derived(Array.from({ length: 42 }, (_, i) => new Date(month.getFullYear(), month.getMonth(), 1 - month.getDay() + i)));

  function toggled(e: ToggleEvent) {
    if (e.newState !== "open") return;
    const d = value ? parseDay(value) : app.today;
    month = new Date(d.getFullYear(), d.getMonth(), 1);
    const r = button.getBoundingClientRect();
    const below = r.bottom + 6 + pop.offsetHeight < innerHeight;
    pop.style.top = `${below ? r.bottom + 6 : r.top - 6 - pop.offsetHeight}px`;
    pop.style.left = `${Math.max(8, Math.min(r.left, innerWidth - pop.offsetWidth - 8))}px`;
    tick().then(() => pop.querySelector<HTMLElement>("[aria-pressed=true], .today")?.focus());
  }
  const shift = (by: number) => month = new Date(month.getFullYear(), month.getMonth() + by, 1);
  function choose(day: string) { value = day; pop.hidePopover(); button.focus(); }
</script>

<button bind:this={button} type="button" class="input date-btn" class:empty={!value} popovertarget={id} aria-haspopup="dialog" aria-label={`${label}: ${text}`}>
  <Icon name="cal" /><span>{text}</span>
</button>
<div bind:this={pop} {id} popover="auto" class="date-pop" role="dialog" aria-label={label} ontoggle={toggled}>
  <div class="cal-h">
    <button type="button" class="btn icon sm ghost" aria-label="Mes anterior" onclick={() => shift(-1)}><Icon name="left" /></button>
    <b>{title}</b>
    <button type="button" class="btn icon sm ghost" aria-label="Mes siguiente" onclick={() => shift(1)}><Icon name="right" /></button>
  </div>
  <div class="grid">
    {#each WEEKDAYS as w (w)}<span class="wd">{w}</span>{/each}
    {#each days as d (d.getTime())}
      {@const iso = isoDay(d)}
      <button type="button" class="d num" class:out={d.getMonth() !== month.getMonth()} class:today={iso === today}
        aria-pressed={iso === value} aria-label={d.toLocaleDateString("es-MX", { dateStyle: "full" })} onclick={() => choose(iso)}>{d.getDate()}</button>
    {/each}
  </div>
  <div class="foot">
    <button type="button" class="btn sm ghost" onclick={() => choose(today)}>Hoy</button>
    {#if optional && value}<button type="button" class="btn sm ghost" onclick={() => choose("")}>Quitar fecha</button>{/if}
  </div>
</div>

<style>
  .date-btn { display: flex; align-items: center; gap: 8px; text-align: left; cursor: pointer; font: inherit; font-weight: 400; color: inherit; }
  .date-btn span::first-letter { text-transform: uppercase; }
  .date-btn.empty { color: var(--muted); }
  .date-btn :global(svg) { color: var(--muted); flex: none; }
  .date-pop { position: fixed; inset: auto; margin: 0; padding: 12px; width: 272px; border: 1px solid var(--border); border-radius: var(--r-panel); background: var(--surface); color: inherit; box-shadow: 0 12px 40px rgb(0 0 0 / .22); }
  .date-pop:popover-open { animation: pop .14s ease-out; }
  .cal-h { display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px; }
  .cal-h b { font-size: 14px; }
  .cal-h b::first-letter { text-transform: uppercase; }
  .grid { display: grid; grid-template-columns: repeat(7, 1fr); gap: 2px; }
  .wd { text-align: center; font-size: 11px; font-weight: 600; color: var(--muted); padding: 4px 0; }
  .d { height: 34px; border: 0; background: none; border-radius: var(--r-control); cursor: pointer; font-size: 13px; color: inherit; }
  .d:hover { background: var(--surface-2); }
  .d.out { color: var(--muted); opacity: .55; }
  .d.today { font-weight: 700; color: var(--brand); }
  .d[aria-pressed="true"] { background: var(--brand); color: var(--brand-ink); font-weight: 650; opacity: 1; }
  .foot { display: flex; justify-content: space-between; margin-top: 8px; padding-top: 8px; border-top: 1px solid var(--border); }
  @media (prefers-reduced-motion: reduce) { .date-pop:popover-open { animation: none; } }
</style>
