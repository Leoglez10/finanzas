<script lang="ts">
  import * as api from "../lib/api";
  import { app, refresh } from "../lib/app.svelte";
  import { categoryColor, money, monthKey, monthTitle, shiftMonth, monthDate } from "../lib/format";
  import { remember, toast } from "../lib/undo.svelte";
  import Icon from "../components/Icon.svelte";
  import Kpi from "../components/Kpi.svelte";
  import Money from "../components/Money.svelte";
  import Topbar from "../components/Topbar.svelte";

  let month = $state(monthKey(app.today));
  let view = $state<api.BudgetView | null>(null);
  let error = $state("");

  $effect(() => {
    void app.version; const m = month;
    api.budget(m).then(v => { view = v; error = ""; }).catch(e => error = String(e));
  });

  let expenseGroups = $derived(view?.groups.filter(g => !g.isIncome) ?? []);
  let incomeGroups = $derived(view?.groups.filter(g => g.isIncome) ?? []);
  let rows = $derived(expenseGroups.flatMap(g => g.categories.filter(c => !c.hidden)));
  let spent = $derived(rows.reduce((s, c) => s + Math.max(0, -c.activity), 0));
  let available = $derived(rows.reduce((s, c) => s + c.available, 0));
  const share = (c: api.BudgetCategory) => c.budgeted > 0 ? Math.max(0, -c.activity) / c.budgeted : (c.activity < 0 ? 2 : 0);
  const status = (c: api.BudgetCategory): [string, string] => c.available < 0 || share(c) > 1 ? ["bad", "Excedido"] : share(c) >= 0.9 ? ["warn", "Casi al límite"] : ["ok", "En camino"];
  const previousName = $derived(monthDate(shiftMonth(month, -1)).toLocaleDateString("es-MX", { month: "long" }));

  async function assign(c: api.BudgetCategory, input: HTMLInputElement) {
    const text = input.value.trim(), before = c.budgeted;
    try {
      const result = await api.assignBudget(month, c.id, text);
      if (result === null) { toast("Escribe un monto válido: 800, +100, -50 o 550*2"); input.value = (before / 100).toFixed(2); return; }
      if (result !== before) remember(`${c.name}: ${money(result)} asignados`, () => api.assignBudget(month, c.id, (before / 100).toFixed(2)).then(refresh));
      await refresh();
    } catch (e) { toast(String(e)); }
  }

  async function copyPrevious() {
    const m = month;
    try {
      const prev = await api.budget(shiftMonth(m, -1));
      const previous = new Map(prev.groups.flatMap(g => g.categories).map(c => [c.id, c.budgeted]));
      const changed = rows.filter(c => previous.has(c.id) && previous.get(c.id) !== c.budgeted).map(c => ({ id: c.id, before: c.budgeted, after: previous.get(c.id)! }));
      if (!changed.length) return toast(`Ya es igual que ${previousName}`);
      for (const c of changed) await api.assignBudget(m, c.id, (c.after / 100).toFixed(2));
      remember(`Copiado de ${previousName}: ${changed.length} categorías`, async () => { for (const c of changed) await api.assignBudget(m, c.id, (c.before / 100).toFixed(2)); await refresh(); });
      await refresh();
    } catch (e) { toast(String(e)); }
  }
</script>

<Topbar title={monthTitle(month)} sub="Presupuesto">
  {#snippet lead()}
    <div class="month">
      <button class="btn icon" aria-label="Mes anterior" onclick={() => month = shiftMonth(month, -1)}><Icon name="left" /></button>
      <button class="btn icon" aria-label="Mes siguiente" onclick={() => month = shiftMonth(month, 1)}><Icon name="right" /></button>
    </div>
  {/snippet}
</Topbar>

{#if error}<p class="err" role="alert">{error}</p>{/if}
{#if view}
  <section class="grid g-3">
    <article class="card hero">
      <p class="label">{view.toBudget >= 0 ? "Disponible para asignar" : "Asignaste de más"}</p>
      <Money cents={Math.abs(view.toBudget)} />
      <p class="note">{view.toBudget >= 0 ? "Dale un destino a cada peso: escribe cuánto va a cada categoría." : "Tus categorías suman más que el dinero disponible. Baja alguna."}</p>
      {#if view.uncategorizedIncome}<p class="note" style="margin-top:6px">Hay {money(view.uncategorizedIncome)} de ingresos sin categoría este mes.</p>{/if}
      <div style="display:flex;gap:8px;margin-top:18px;position:relative;z-index:1">
        <button class="btn" style="background:#fff;color:var(--hero);border:0" onclick={copyPrevious}>Copiar de {previousName}</button>
      </div>
    </article>
    <div class="grid" style="gap:16px">
      <Kpi icon="pie" label="Asignado" cents={view.assigned} />
      <Kpi icon="down" tone="bad" label="Gastado" cents={spent} note={view.assigned ? `${(spent / view.assigned * 100).toFixed(0)}% de lo asignado` : ""} />
    </div>
  </section>

  <p class="muted" style="font-size:13px">Escribe cuánto asignas, o usa <b>+100</b>, <b>-50</b> o <b>550*2</b>. Enter para guardar.</p>
  <article class="card">
    <div class="budget-row head"><span>Categoría</span><span class="hide-narrow">Progreso</span><span style="text-align:right">Asignado</span><span class="hide-narrow" style="text-align:right">Gastado</span><span style="text-align:right">Disponible</span></div>
    {#each expenseGroups as g (g.id)}
      <div class="budget-group">{g.name}<span class="num">{money(g.categories.reduce((s, c) => s + c.available, 0))} disponibles</span></div>
      {#each g.categories.filter(c => !c.hidden) as c (c.id)}
        {@const [tone, label] = status(c)}
        <div class="budget-row" style="--c:{categoryColor(c.id)}">
          <span class="name"><i></i>{c.name}</span>
          <span class="hide-narrow" style="display:grid;gap:5px">
            <span class="bar"><span style="width:{Math.min(100, share(c) * 100)}%"></span></span>
            <span class="chip {tone}" style="justify-self:start">{label}{c.budgeted ? ` · ${(share(c) * 100).toFixed(0)}%` : ""}</span>
          </span>
          {#key c.budgeted}
            <input class="input num" value={(c.budgeted / 100).toFixed(2)} inputmode="decimal" aria-label="Asignado a {c.name}"
              onkeydown={e => { if (e.key === "Enter") e.currentTarget.blur(); if (e.key === "Escape") { e.currentTarget.value = (c.budgeted / 100).toFixed(2); e.currentTarget.blur(); } }}
              onchange={e => assign(c, e.currentTarget)} />
          {/key}
          <span class="num hide-narrow" style="text-align:right">{money(Math.max(0, -c.activity))}</span>
          <b class="num" class:neg={c.available < 0} class:pos={c.available >= 0} style="text-align:right">{money(c.available)}</b>
        </div>
      {/each}
    {:else}
      <p class="muted" style="padding:12px 6px">Aún no tienes categorías de gasto.</p>
    {/each}
    {#each incomeGroups as g (g.id)}
      <div class="budget-group" style="border-top:1px solid var(--border);margin-top:6px">{g.name}<span class="num pos">{money(view.income)}</span></div>
      {#each g.categories.filter(c => !c.hidden) as c (c.id)}
        <div class="budget-row" style="--c:var(--c-sueldo)"><span class="name"><i></i>{c.name}</span><span class="hide-narrow"></span><span></span><span class="hide-narrow"></span><b class="num pos" style="text-align:right">{money(c.activity)}</b></div>
      {/each}
    {/each}
  </article>
  <p class="muted" style="font-size:13px">Disponible total en categorías: <b class="num">{money(available)}</b></p>
{/if}
