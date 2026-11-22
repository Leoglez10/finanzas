<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import * as api from "../lib/api";
  import { app, refresh } from "../lib/app.svelte";
  import { openWish } from "../lib/dialogs.svelte";
  import { isoDay, money, parseDay } from "../lib/format";
  import { parseCents } from "../lib/quick-parse";
  import { remember, toast } from "../lib/undo.svelte";
  import { kindOf, planText, runningTotal } from "../lib/goals";
  import Icon from "../components/Icon.svelte";
  import Kpi from "../components/Kpi.svelte";
  import Money from "../components/Money.svelte";
  import Topbar from "../components/Topbar.svelte";

  let showArchived = $state(false);
  let views = $state<api.WishView[]>([]);
  let groups = $state<api.CategoryGroup[]>([]);
  let reserving = $state<number | null>(null);
  let error = $state("");

  $effect(() => {
    void app.version; const archived = showArchived;
    Promise.all([api.wishes(archived), api.categoriesGrouped()]).then(([w, g]) => { views = archived ? w.filter(v => v.wish.archived) : w; groups = g; error = ""; }).catch(e => error = String(e));
  });
  let categoryName = (id: number | null) => groups.flatMap(g => g.categories).find(c => c.id === id)?.name ?? "Sin categoría";
  let active = $derived(views.filter(v => !v.wish.archived));
  const STATUS: Record<string, [string, string]> = {
    affordable: ["brand", "Cabe en tu presupuesto y saldo"],
    budget_shortfall: ["warn", "Falta presupuesto"],
    cash_shortfall: ["warn", "Tu saldo no cubre la compra y los pagos próximos"],
    reservation_deficit: ["bad", "Hay reservas sin respaldo en esta categoría"],
    budget_overassigned: ["warn", "Tu presupuesto tiene dinero asignado de más"],
  };
  function verdict(v: api.WishView): [string, string] {
    if (v.wish.reserved + v.wish.paid >= v.wish.price) return ["ok", v.wish.kind === "compra" ? "Listo para comprar" : "Meta completa"];
    const [tone, text] = STATUS[v.evaluation.status] ?? ["", "Faltan datos para evaluarlo"];
    const gap = v.evaluation.status === "budget_shortfall" ? v.evaluation.budgetShortfall : v.evaluation.status === "cash_shortfall" ? v.evaluation.cashShortfall : null;
    return [tone, gap ? `${text}: faltan ${money(gap)}` : text];
  }

  async function act(label: string, run: () => Promise<() => Promise<unknown>>) {
    try { const revert = await run(); remember(label, () => revert().then(refresh)); await refresh(); }
    catch (e) { toast(api.errorText(e)); }
  }
  const setReserved = (w: api.Wish, total: number, label: string) => act(label, async () => {
    const before = w.reserved; await api.reserveWish(w.id, total); return () => api.reserveWish(w.id, before);
  });
  function addReserve(w: api.Wish, text: string) {
    const cents = parseCents(text);
    if (!cents || cents <= 0) return toast("Escribe un monto válido, por ejemplo 200 o 1,250.50");
    reserving = null;
    const total = Math.min(w.price, w.reserved + cents);
    void setReserved(w, total, `Apartaste ${money(total - w.reserved)} para ${w.name}`);
  }
  // Paying an item can release money set aside; undo unmarks it and puts the exact snapshot back.
  const togglePaid = (w: api.Wish, it: api.WishItem, paid: boolean) => act(`${it.name}: ${paid ? "pagado" : "pendiente"}`, async () => {
    await api.setWishItemPaid(it.id, paid);
    return () => api.setWishItemPaid(it.id, !paid).then(() => api.restoreWish(w));
  });
  /** Step line of the set-aside total, as SVG points in a 100×40 box (full height = what is left to cover once paid items are out). */
  function spark(log: api.WishContribution[], price: number) {
    const pts = runningTotal(log), y = (t: number) => (40 - Math.max(0, Math.min(1, t / price)) * 36).toFixed(1);
    let last = "40", out = "0,40";
    pts.forEach((p, i) => { const x = ((i + 1) / pts.length * 100).toFixed(1); out += ` ${x},${last} ${x},${y(p.total)}`; last = y(p.total); });
    return out;
  }
  const shortDay = (iso: string) => parseDay(iso).toLocaleDateString("es-MX", { day: "numeric", month: "short" }).replace(".", "");
  const archive = (w: api.Wish) => act(`Archivado: ${w.name}`, async () => { await api.archiveWish(w.id); return () => api.restoreWish(w); });
  // Buying records the expense and archives the wish; undo removes both changes.
  // Completing records the expense(s) and keeps the goal as an achievement; undo removes both changes.
  const complete = (w: api.Wish) => act(`${kindOf(w.kind).done}: ${w.name} ${money(-(w.price - w.paid))}`, async () => {
    const account = app.accounts.find(a => !a.closed && !a.offBudget) ?? app.accounts.find(a => !a.closed);
    if (!account) throw new Error("Primero crea una cuenta");
    const done = await api.completeWish(w.id, account.id);
    return async () => { for (const e of done.entries) await api.deleteEntry(e.id); await api.restoreWish(w); };
  });
</script>

<Topbar title="Deseos y metas" sub="Aparta dinero para compras, viajes y más, sin registrar un gasto">
  {#snippet actions()}
    <button class="btn" aria-pressed={showArchived} onclick={() => showArchived = !showArchived}><Icon name="archive" />{showArchived ? "Ver activos" : "Archivados"}</button>
    <button class="btn primary" onclick={() => openWish()}><Icon name="plus" />Nueva meta</button>
  {/snippet}
</Topbar>

{#if error}<p class="err" role="alert">{error}</p>{/if}
{#if !showArchived}
  <section class="kpis">
    <Kpi icon="gift" tone="violet" label="Total de metas" cents={active.reduce((s, v) => s + v.wish.price, 0)} note="{active.length} meta{active.length === 1 ? '' : 's'}" />
    <Kpi icon="wallet" label="Ya apartado" cents={active.reduce((s, v) => s + v.wish.reserved, 0)} />
    <Kpi icon="cal" label="Aparta este mes" cents={active.reduce((s, v) => s + (v.plan && !v.plan.overdue ? v.plan.monthly : 0), 0)} note="Según las fechas de tus metas" />
    <Kpi icon="check" tone="ok" label="Listas" value={String(active.filter(v => v.wish.reserved + v.wish.paid >= v.wish.price).length)} />
  </section>
{/if}

<section class="wishes">
  {#each views as v (v.wish.id)}
    {@const w = v.wish}{@const k = kindOf(w.kind)}{@const pct = Math.min(100, (w.reserved + w.paid) / w.price * 100)}{@const [tone, text] = verdict(v)}
    <article class="card wish">
      <div class="wish-top"><div class="ic violet" title={k.label}><Icon name={k.icon} /></div>
        <div style="flex:1;min-width:0"><h3>{w.name}</h3><p class="muted" style="font-size:13px">{k.label} · {categoryName(w.categoryId)}{w.notes ? ` · ${w.notes}` : ""}</p></div></div>
      <Money cents={w.price} class="price" />
      {#if w.archived}
        {#if w.achievedDate}<span class="chip ok"><Icon name="star" />Logro · {k.done.toLowerCase()} el {shortDay(w.achievedDate)} {w.achievedDate.slice(0, 4)}</span>
        {:else}<span class="chip">Archivado</span>{/if}
      {:else}
        <div style="display:grid;gap:6px">
          <div class="bar" style="--c:var(--violet)"><span style="width:{pct}%"></span></div>
          <div style="display:flex;justify-content:space-between;font-size:12.5px"><span class="muted">{w.paid ? `Pagado ${money(w.paid)} · ` : ""}Apartado {money(w.reserved)} de {money(w.price)}</span><b class="num">{pct.toFixed(0)}%</b></div>
        </div>
        {#if v.items.length}
          <ul class="items" aria-label="Partidas de {w.name}">
            {#each v.items as it (it.id)}
              {@const spent = v.spending.items.find(x => x.itemId === it.id)?.spent ?? 0}
              <li><label><input type="checkbox" checked={it.paid} onchange={e => togglePaid(w, it, e.currentTarget.checked)} /><span class:done={it.paid}>{it.name}</span></label>
                <span class="num">{#if spent}<span class:over={spent > it.amount} class="spent">gastado {money(spent)} de </span>{/if}<b>{money(it.amount)}</b></span></li>
            {/each}
          </ul>
        {/if}
        {#if v.spending.total}
          <p class="spent-total"><span class="muted">Gastado (movimientos etiquetados)</span><b class="num" class:over={v.spending.total > w.price}>{money(v.spending.total)} de {money(w.price)}</b></p>
          {#if v.spending.other}<p class="spent-total"><span class="muted">Sin partida</span><span class="num">{money(v.spending.other)}</span></p>{/if}
        {/if}
        {#if v.contributions.length}
          <details class="history">
            <summary>Historial · {v.contributions.length} movimiento{v.contributions.length === 1 ? "" : "s"}</summary>
            {#if runningTotal(v.contributions).length > 1}<svg class="spark" viewBox="0 0 100 40" preserveAspectRatio="none" role="img" aria-label="Avance de lo apartado para {w.name}"><polyline points={spark(v.contributions, Math.max(1, w.price - w.paid))} /></svg>{/if}
            <ul>{#each v.contributions.slice(-5).reverse() as c, i (i)}<li><span class="muted">{shortDay(c.date)}</span><b class="num" class:pos={c.amount > 0}>{c.amount > 0 ? "Apartaste" : "Liberaste"} {money(Math.abs(c.amount))}</b></li>{/each}</ul>
          </details>
        {/if}
        {#if v.plan}<p class="plan" class:late={v.plan.overdue}><Icon name={v.plan.overdue ? "alert" : "cal"} />{planText(v.plan)}</p>{/if}
        <div><span class="chip {tone}">{text}</span></div>
        {#if reserving === w.id}
          <form class="reserve" onsubmit={e => { e.preventDefault(); addReserve(w, (e.currentTarget.elements.namedItem("amount") as HTMLInputElement).value); }}>
            <!-- svelte-ignore a11y_autofocus (opened by the user's own click on "Apartar más") -->
            <input class="input" name="amount" inputmode="decimal" placeholder={money(Math.max(0, w.price - w.reserved))} aria-label="Monto a apartar para {w.name}" autofocus />
            <button class="btn primary">Apartar</button><button class="btn ghost" type="button" onclick={() => reserving = null}>Cancelar</button>
          </form>
        {:else}
          <div class="foot">
            {#if w.reserved + w.paid >= w.price}<button class="btn sm primary" onclick={() => complete(w)}><Icon name="check" />{k.done}</button>
            {:else}<button class="btn sm primary" onclick={() => reserving = w.id}>Apartar más</button>{/if}
            {#if w.reserved + w.paid < w.price}<button class="btn sm ghost" title="Registra el gasto y la guarda como logro" onclick={() => complete(w)}><Icon name="check" />Cumplida</button>{/if}
            {#if w.reserved}<button class="btn sm" onclick={() => setReserved(w, 0, `Liberaste ${money(w.reserved)} de ${w.name}`)}>Liberar</button>{/if}
            {#if w.url}<button class="btn sm ghost" onclick={() => openUrl(w.url).catch(e => toast(String(e)))}>{w.kind === "compra" ? "Ver producto" : "Ver enlace"}</button>{/if}
            <span class="spacer"></span>
            <button class="btn icon sm ghost" aria-label="Editar {w.name}" onclick={() => openWish(w, v.items)}><Icon name="edit" /></button>
            <button class="btn icon sm ghost" aria-label="Archivar {w.name}" onclick={() => archive(w)}><Icon name="archive" /></button>
          </div>
        {/if}
      {/if}
    </article>
  {/each}
  {#if !showArchived}
    <button class="card empty-add" onclick={() => openWish()}><Icon name="plus" /><b>Agregar una meta</b><span style="font-size:13px">Una compra, un viaje, una experiencia o un fondo</span></button>
  {:else if !views.length}<p class="muted">No tienes metas archivadas.</p>{/if}
</section>

<style>
  .plan { display: flex; align-items: center; gap: 6px; margin: 0; font-size: 13px; font-weight: 550; color: var(--brand); }
  .plan.late { color: var(--expense); }
  .items { list-style: none; margin: 0; padding: 0; display: grid; gap: 4px; font-size: 13px; }
  .items li { display: flex; justify-content: space-between; gap: 8px; align-items: center; }
  .items label { display: inline-flex; align-items: center; gap: 8px; cursor: pointer; }
  .history summary { cursor: pointer; font-size: 12.5px; color: var(--muted); font-weight: 550; }
  .history ul { list-style: none; margin: 6px 0 0; padding: 0; display: grid; gap: 2px; font-size: 12.5px; }
  .history li { display: flex; justify-content: space-between; }
  .spark { display: block; width: 100%; height: 40px; margin-top: 6px; }
  .spark polyline { fill: none; stroke: var(--violet); stroke-width: 1.5; vector-effect: non-scaling-stroke; }
  .spent { color: var(--muted); font-size: 12px; }
  .over { color: var(--expense) !important; }
  .spent-total { display: flex; justify-content: space-between; margin: 0; font-size: 12.5px; }
  .items .done { text-decoration: line-through; color: var(--muted); }
</style>
