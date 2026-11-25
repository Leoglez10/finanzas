<script lang="ts">
  import { untrack } from "svelte";
  import * as api from "../lib/api";
  import { app, go, refresh } from "../lib/app.svelte";
  import { openEntry } from "../lib/dialogs.svelte";
  import { showMenu, type MenuEntry } from "../lib/contextmenu";
  import { runCommand } from "../lib/commands.svelte";
  import { categoryColor, money, parseDay } from "../lib/format";
  import { rangeOfParams, inRange, type Range } from "../lib/period";
  import { norm, parseCents } from "../lib/quick-parse";
  import { remember, toast } from "../lib/undo.svelte";
  import Icon from "../components/Icon.svelte";
  import Kpi from "../components/Kpi.svelte";
  import PeriodPicker from "../components/PeriodPicker.svelte";
  import Topbar from "../components/Topbar.svelte";
  import AccountBadge from "../components/AccountBadge.svelte";
  import { openAccountStyle } from "../lib/dialogs.svelte";

  let { accountId = 0 }: { accountId?: number } = $props();
  // Filters live in the route: movimientos?desde=2026-11-01&hasta=2026-11-30&categoria=4 (mes=202611 still works).
  let params = $derived(new URLSearchParams(app.route.split("?")[1] ?? ""));
  let search = $state(new URLSearchParams(location.hash.split("?")[1] ?? "").get("q") ?? "");
  // movimientos?tipo=gastos opens with that filter (the Resumen cards link here).
  let kind = $state<"todos" | "gastos" | "ingresos">((["gastos", "ingresos"] as const).find(k => k === new URLSearchParams(location.hash.split("?")[1] ?? "").get("tipo")) ?? "todos");
  let range = $derived(rangeOfParams(params));
  let categoryFilter = $derived(Number(params.get("categoria")) || 0);
  let rows = $state<api.EntryRow[]>([]);
  let groups = $state<api.CategoryGroup[]>([]);
  let goalNames = $state(new Map<number, string>());
  let editing = $state<{ id: number; field: "amount" | "category" } | null>(null);
  let error = $state("");

  $effect(() => {
    void app.version; const id = accountId;
    // ponytail: loads the whole ledger and filters here; page it in SQL when ledgers pass ~20k rows.
    Promise.all([api.entries({ accountId: id || undefined }), api.categoriesGrouped(), api.wishes(true)])
      .then(([r, g, w]) => { rows = r.rows; groups = g; goalNames = new Map(w.map(v => [v.wish.id, v.wish.name])); error = ""; }).catch(e => error = String(e));
  });

  let account = $derived(app.accounts.find(a => a.id === accountId));
  let categories = $derived(groups.flatMap(g => g.categories));
  let categoryName = $derived(categories.find(c => c.id === categoryFilter)?.name);
  let visible = $derived.by(() => {
    const q = norm(search.trim());
    return rows.filter(r =>
      (kind === "todos" || (kind === "gastos") === (r.amount < 0)) &&
      inRange(r.date, range) &&
      (!categoryFilter || r.categoryId === categoryFilter) &&
      (!q || norm([r.payeeName, r.categoryName, r.notes, r.accountName].join(" ")).includes(q)));
  });
  let days = $derived(Map.groupBy(visible, r => r.date));
  let inflow = $derived(visible.filter(r => r.amount > 0).reduce((s, r) => s + r.amount, 0));
  let outflow = $derived(-visible.filter(r => r.amount < 0).reduce((s, r) => s + r.amount, 0));
  let balance = $derived(account ? account.balance : app.accounts.filter(a => !a.closed).reduce((s, a) => s + a.balance, 0));
  const dayTotal = (list: api.EntryRow[]) => list.reduce((s, r) => s + r.amount, 0);
  const longDay = (iso: string) => parseDay(iso).toLocaleDateString("es-MX", { weekday: "long", day: "numeric", month: "long", year: parseDay(iso).getFullYear() === app.today.getFullYear() ? undefined : "numeric" });
  const initials = (s: string | null) => (s ?? "?").split(/\s+/).slice(0, 2).map(w => w[0]).join("").toUpperCase();
  const navigate = (p: URLSearchParams) => { const rest = p.toString(); go((accountId ? `cuenta-${accountId}` : "movimientos") + (rest ? `?${rest}` : "")); };
  const clearFilter = (key: string) => { const p = new URLSearchParams(params); p.delete(key); navigate(p); };
  function setRange(r: Range) {
    const p = new URLSearchParams(params); p.delete("mes");
    for (const [key, value] of [["desde", r.from], ["hasta", r.to]]) { if (value) p.set(key, value); else p.delete(key); }
    navigate(p);
  }

  async function change(row: api.EntryRow, patch: Partial<api.EntryInput>) {
    const before = api.inputOf(row);
    try {
      await api.updateEntry(row.id, { ...before, ...patch });
      remember("Movimiento actualizado", () => api.updateEntry(row.id, before).then(refresh));
      await refresh();
    } catch (e) { toast(String(e)); }
  }
  const kindOfRow = (r: api.EntryRow) => r.amount < 0 ? "gasto" as const : "ingreso" as const;
  const rowMenu = (r: api.EntryRow): MenuEntry[] => [
    ["Editar…", () => openEntry(kindOfRow(r), r)],
    ["Duplicar con fecha de hoy…", () => openEntry(kindOfRow(r), null, { cents: Math.abs(r.amount), payee: r.payeeName ?? "", categoryId: r.categoryId, accountId: r.accountId })],
    r.categoryId !== null && [`Ver solo ${r.categoryName ?? "esta categoría"}`, () => go(`movimientos?categoria=${r.categoryId}`)],
    r.accountId !== null && !accountId && [`Abrir ${r.accountName ?? "la cuenta"}`, () => go(`cuenta-${r.accountId}`)],
    null,
    ["Eliminar", () => remove(r)],
  ];
  async function remove(row: api.EntryRow) {
    const before = api.inputOf(row);
    try {
      await api.deleteEntry(row.id);
      // ponytail: undo re-adds the movement with a new id; a link to its scheduled payment is not restored.
      remember(`Eliminado: ${row.payeeName ?? "movimiento"}`, () => api.addEntry(before).then(refresh));
      await refresh();
    } catch (e) { toast(String(e)); }
  }
  function commitAmount(row: api.EntryRow, text: string) {
    editing = null;
    const cents = parseCents(text);
    if (!cents) return toast("Monto no válido: usa 1,250.50");
    if (cents !== row.amount) void change(row, { amount: cents });
  }
  function startEdit(id: number, field: "amount" | "category") { editing = { id, field }; }
  function focusOnMount(node: HTMLInputElement | HTMLSelectElement) { untrack(() => { node.focus(); if (node instanceof HTMLInputElement) node.select(); }); }
</script>

<Topbar title={account?.name ?? "Movimientos"} sub={account ? "Cuenta" : "Todas las cuentas"}>
  {#snippet lead()}{#if account}<button class="badge-btn" title="Cambiar apariencia" aria-label="Cambiar apariencia de {account.name}" onclick={() => openAccountStyle(account!.id)}><AccountBadge {account} size={40} /></button>{/if}{/snippet}
  {#snippet actions()}
    {#if account}<button class="btn ghost" onclick={() => openAccountStyle(account!.id)}><Icon name="edit" />Apariencia</button>{/if}
    <button class="btn" onclick={() => runCommand("import")}><Icon name="upload" />Importar</button>
    <button class="btn" onclick={() => openEntry("ingreso", null, { accountId: accountId || null })}><Icon name="plus" />Ingreso</button>
    <button class="btn primary" onclick={() => openEntry("gasto", null, { accountId: accountId || null })}><Icon name="minus" />Gasto</button>
  {/snippet}
</Topbar>

<section class="kpis">
  <Kpi label={account ? "Saldo de la cuenta" : "Saldo total"} cents={balance} onclick={() => kind = "todos"} hint="Ver todos los movimientos" />
  <Kpi tone="ok" label="Entradas" cents={inflow} onclick={() => kind = "ingresos"} hint="Ver solo las entradas" />
  <Kpi tone="bad" label="Salidas" cents={outflow} onclick={() => kind = "gastos"} hint="Ver solo las salidas" />
</section>

<div class="toolbar">
  <label class="search"><Icon name="search" /><input class="input" type="search" bind:value={search} placeholder="Buscar comercio, categoría o nota" aria-label="Buscar movimientos" /></label>
  <div class="seg" role="group" aria-label="Tipo">
    {#each [["todos", "Todos"], ["gastos", "Gastos"], ["ingresos", "Ingresos"]] as [value, label] (value)}
      <button aria-pressed={kind === value} onclick={() => kind = value as typeof kind}>{label}</button>
    {/each}
  </div>
  <PeriodPicker {range} today={app.today} onchange={setRange} />
  {#if categoryFilter}<button class="chip brand" onclick={() => clearFilter("categoria")} aria-label="Quitar filtro de categoría">{categoryName ?? "Categoría"} <Icon name="x" /></button>{/if}
  <span class="muted" style="margin-left:auto;font-size:13px">{visible.length} movimientos · doble clic en monto o categoría para editar</span>
</div>

{#if error}<p class="err" role="alert">{error}</p>{/if}
<!-- svelte-ignore a11y_no_noninteractive_tabindex (a scrollable region must be keyboard-focusable to scroll) -->
<div class="table-wrap" tabindex="0" role="region" aria-labelledby="ledger-cap">
  <table>
    <caption id="ledger-cap" class="sr">Movimientos</caption>
    <thead><tr><th>Comercio</th><th>Categoría</th>{#if !accountId}<th class="hide-narrow">Cuenta</th>{/if}<th class="hide-narrow">Notas</th><th class="r">Monto</th><th class="r"><span class="sr">Acciones</span></th></tr></thead>
    {#each days as [date, list] (date)}
      <tbody>
        <tr><th colspan={accountId ? 5 : 6} class="day" scope="rowgroup">{longDay(date)}<span class="num" class:pos={dayTotal(list) > 0}>{money(dayTotal(list), true)}</span></th></tr>
        {#each list as r (r.id)}
          <tr class="tx" oncontextmenu={e => showMenu(e, rowMenu(r))}>
            <td><div class="who"><span class="merchant" style="--c:{categoryColor(r.categoryId)}">{initials(r.payeeName)}</span><b><button class="edit-name" title="Editar movimiento" onclick={() => openEntry(r.amount < 0 ? "gasto" : "ingreso", r)}>{r.payeeName ?? "Sin comercio"}</button></b>{#if r.wishId}<span class="chip violet" title="Etiquetado con una meta"><Icon name="star" />{goalNames.get(r.wishId) ?? "Meta"}</span>{/if}</div></td>
            <td data-edit ondblclick={() => startEdit(r.id, "category")} title="Doble clic para cambiar">
              {#if editing?.id === r.id && editing?.field === "category"}
                <select class="input inline" aria-label="Categoría de {r.payeeName ?? 'movimiento'}" use:focusOnMount
                  onchange={e => { editing = null; const v = (e.currentTarget as HTMLSelectElement).value; void change(r, { categoryId: v ? Number(v) : null }); }}
                  onblur={() => editing = null} onkeydown={e => { if (e.key === "Escape") { e.preventDefault(); editing = null; } }}>
                  <option value="" selected={r.categoryId === null}>Sin categoría</option>
                  {#each groups as g (g.id)}<optgroup label={g.name}>{#each g.categories as c (c.id)}<option value={c.id} selected={c.id === r.categoryId}>{c.name}</option>{/each}</optgroup>{/each}
                </select>
              {:else}
                <span class="chip" style="--c:{categoryColor(r.categoryId)}"><span class="dot"></span>{r.categoryName ?? "Sin categoría"}</span>
              {/if}
            </td>
            {#if !accountId}<td class="hide-narrow muted">{#if r.accountId !== null}<button class="text-link" title="Abrir la cuenta" onclick={() => go(`cuenta-${r.accountId}`)}>{r.accountName}</button>{/if}</td>{/if}
            <td class="hide-narrow muted">{r.notes}</td>
            <td class="r amt" class:pos={r.amount > 0} data-edit ondblclick={() => startEdit(r.id, "amount")} title="Doble clic para editar">
              {#if editing?.id === r.id && editing?.field === "amount"}
                <input class="input inline num" value={(r.amount / 100).toFixed(2)} inputmode="decimal" aria-label="Monto de {r.payeeName ?? 'movimiento'}" use:focusOnMount
                  onkeydown={e => { if (e.key === "Enter") commitAmount(r, e.currentTarget.value); if (e.key === "Escape") { e.preventDefault(); editing = null; } }}
                  onblur={e => { if (editing) commitAmount(r, e.currentTarget.value); }} />
              {:else}{money(r.amount, true)}{/if}
            </td>
            <td class="r"><span class="row-actions">
              <button class="btn icon sm ghost" aria-label="Editar {r.payeeName ?? 'movimiento'}" onclick={() => openEntry(r.amount < 0 ? "gasto" : "ingreso", r)}><Icon name="edit" /></button>
              <button class="btn icon sm ghost" aria-label="Eliminar {r.payeeName ?? 'movimiento'}" onclick={() => remove(r)}><Icon name="trash" /></button>
            </span></td>
          </tr>
        {/each}
      </tbody>
    {:else}
      <tbody><tr><td colspan={accountId ? 5 : 6} style="text-align:center;padding:48px" class="muted">{rows.length ? "Sin movimientos con estos filtros." : "Aún no hay movimientos. Agrega el primero con “Gasto” o “Ingreso”."}</td></tr></tbody>
    {/each}
  </table>
</div>

<style>
  .badge-btn { all: unset; cursor: pointer; border-radius: var(--r-panel); }
  .badge-btn:focus-visible { outline: 2px solid var(--brand); outline-offset: 2px; }
</style>
