<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import * as api from "../lib/api";
  import { app, refresh } from "../lib/app.svelte";
  import { openWish } from "../lib/dialogs.svelte";
  import { isoDay, money } from "../lib/format";
  import { parseCents } from "../lib/quick-parse";
  import { remember, toast } from "../lib/undo.svelte";
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
    if (v.wish.reserved >= v.wish.price) return ["ok", "Listo para comprar"];
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
  const archive = (w: api.Wish) => act(`Archivado: ${w.name}`, async () => { await api.archiveWish(w.id); return () => api.restoreWish(w); });
  // Buying records the expense and archives the wish; undo removes both changes.
  const buy = (w: api.Wish) => act(`Comprado: ${w.name} ${money(-w.price)}`, async () => {
    const account = app.accounts.find(a => !a.closed && !a.offBudget) ?? app.accounts.find(a => !a.closed);
    if (!account) throw new Error("Primero crea una cuenta");
    const entry = await api.addEntry({ date: isoDay(app.today), amount: -w.price, accountId: account.id, payee: w.name, categoryId: w.categoryId, notes: "Compra de un deseo" });
    await api.archiveWish(w.id);
    return async () => { await api.deleteEntry(entry.id); await api.restoreWish(w); };
  });
</script>

<Topbar title="Deseos" sub="Aparta dinero sin registrar un gasto">
  {#snippet actions()}
    <button class="btn" aria-pressed={showArchived} onclick={() => showArchived = !showArchived}><Icon name="archive" />{showArchived ? "Ver activos" : "Archivados"}</button>
    <button class="btn primary" onclick={() => openWish()}><Icon name="plus" />Nuevo deseo</button>
  {/snippet}
</Topbar>

{#if error}<p class="err" role="alert">{error}</p>{/if}
{#if !showArchived}
  <section class="kpis">
    <Kpi icon="gift" tone="violet" label="Total de deseos" cents={active.reduce((s, v) => s + v.wish.price, 0)} note="{active.length} deseo{active.length === 1 ? '' : 's'}" />
    <Kpi icon="wallet" label="Ya apartado" cents={active.reduce((s, v) => s + v.wish.reserved, 0)} />
    <Kpi icon="check" tone="ok" label="Listos para comprar" value={String(active.filter(v => v.wish.reserved >= v.wish.price).length)} />
  </section>
{/if}

<section class="wishes">
  {#each views as v (v.wish.id)}
    {@const w = v.wish}{@const pct = Math.min(100, w.reserved / w.price * 100)}{@const [tone, text] = verdict(v)}
    <article class="card wish">
      <div class="wish-top"><div class="ic violet"><Icon name="gift" /></div>
        <div style="flex:1;min-width:0"><h3>{w.name}</h3><p class="muted" style="font-size:13px">{categoryName(w.categoryId)}{w.notes ? ` · ${w.notes}` : ""}</p></div></div>
      <Money cents={w.price} class="price" />
      {#if w.archived}
        <span class="chip">Archivado</span>
      {:else}
        <div style="display:grid;gap:6px">
          <div class="bar" style="--c:var(--violet)"><span style="width:{pct}%"></span></div>
          <div style="display:flex;justify-content:space-between;font-size:12.5px"><span class="muted">Apartado {money(w.reserved)} de {money(w.price)}</span><b class="num">{pct.toFixed(0)}%</b></div>
        </div>
        <div><span class="chip {tone}">{text}</span></div>
        {#if reserving === w.id}
          <form class="reserve" onsubmit={e => { e.preventDefault(); addReserve(w, (e.currentTarget.elements.namedItem("amount") as HTMLInputElement).value); }}>
            <!-- svelte-ignore a11y_autofocus (opened by the user's own click on "Apartar más") -->
            <input class="input" name="amount" inputmode="decimal" placeholder={money(Math.max(0, w.price - w.reserved))} aria-label="Monto a apartar para {w.name}" autofocus />
            <button class="btn primary">Apartar</button><button class="btn ghost" type="button" onclick={() => reserving = null}>Cancelar</button>
          </form>
        {:else}
          <div class="foot">
            {#if w.reserved >= w.price}<button class="btn sm primary" onclick={() => buy(w)}><Icon name="check" />Lo compré</button>
            {:else}<button class="btn sm primary" onclick={() => reserving = w.id}>Apartar más</button>{/if}
            {#if w.reserved}<button class="btn sm" onclick={() => setReserved(w, 0, `Liberaste ${money(w.reserved)} de ${w.name}`)}>Liberar</button>{/if}
            {#if w.url}<button class="btn sm ghost" onclick={() => openUrl(w.url).catch(e => toast(String(e)))}>Ver producto</button>{/if}
            <span class="spacer"></span>
            <button class="btn icon sm ghost" aria-label="Editar {w.name}" onclick={() => openWish(w)}><Icon name="edit" /></button>
            <button class="btn icon sm ghost" aria-label="Archivar {w.name}" onclick={() => archive(w)}><Icon name="archive" /></button>
          </div>
        {/if}
      {/if}
    </article>
  {/each}
  {#if !showArchived}
    <button class="card empty-add" onclick={() => openWish()}><Icon name="plus" /><b>Agregar un deseo</b><span style="font-size:13px">Guarda algo que quieras comprar</span></button>
  {:else if !views.length}<p class="muted">No tienes deseos archivados.</p>{/if}
</section>
