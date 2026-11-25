<script lang="ts">
  import * as api from "../lib/api";
  import { app, go, refresh } from "../lib/app.svelte";
  import { openCard, openInstallment } from "../lib/dialogs.svelte";
  import { advice, looksLikeCard, missingText } from "../lib/debts";
  import { money, parseDay } from "../lib/format";
  import { remember, toast } from "../lib/undo.svelte";
  import Icon from "../components/Icon.svelte";
  import AccountBadge from "../components/AccountBadge.svelte";
  import Kpi from "../components/Kpi.svelte";
  import Topbar from "../components/Topbar.svelte";

  let view = $state<api.DebtsView | null>(null);
  let error = $state("");
  $effect(() => { void app.version; api.debts().then(v => { view = v; error = ""; }).catch(e => error = String(e)); });
  let tips = $derived(view ? advice(view.debts) : []);
  let cards = $derived(view?.debts.filter(d => d.isCard) ?? []);
  let others = $derived(view?.debts.filter(d => !d.isCard) ?? []);
  let withStatement = $derived(cards.some(d => d.statement !== null));
  let toPay = $derived(cards.reduce((s, d) => s + (d.toAvoidInterest ?? 0), 0));
  let hasPlans = $derived(cards.some(d => d.installments.some(p => p.left > 0)));
  const look = (id: number) => app.accounts.find(a => a.id === id) ?? { offBudget: false, icon: "card", color: null, image: null };
  const sum = (list: api.DebtView[]) => list.reduce((s, d) => s + d.debt, 0);
  const date = (iso: string) => parseDay(iso).toLocaleDateString("es-MX", { day: "numeric", month: "short" });

  async function remove(p: api.Installment) {
    try {
      const removed = await api.deleteInstallment(p.id);
      remember(`Quitaste “${removed.name}”`, () => api.saveInstallment(null, removed).then(refresh));
      await refresh();
    } catch (e) { toast(String(e)); }
  }
</script>

{#snippet group(title: string, total: number)}
  <div class="group"><h2>{title}</h2><span class="num muted">{money(total)}</span></div>
{/snippet}

<Topbar title="Deudas" sub="Todo lo que debes, en un lugar">
  {#snippet actions()}<button class="btn primary" onclick={() => openCard()}><Icon name="plus" />Agregar tarjeta</button>{/snippet}
</Topbar>

{#if error}<p class="err" role="alert">{error}</p>{/if}
{#if view}
  <section class="kpis">
    <Kpi icon="card" tone="bad" label="Debes en total" cents={view.total} note="{view.debts.length} deuda{view.debts.length === 1 ? '' : 's'}" />
    {#if withStatement}<Kpi icon="clock" tone="warn" label="Para no pagar intereses" cents={toPay} note="Suma de este corte" />
    {:else}<Kpi icon="clock" tone="warn" label="Para no pagar intereses" value="—" note="Agrega el día de corte de una tarjeta para verlo" />{/if}
    {#if hasPlans}<Kpi icon="repeat" tone="ok" label="Mensualidades a meses" cents={view.monthlyInstallments} />
    {:else}<Kpi icon="repeat" tone="ok" label="Mensualidades a meses" value="—" note="Sin compras a meses" />{/if}
  </section>

  {#if tips.length}
    <article class="card">
      <div class="card-h"><h2>Qué te recomiendo</h2></div>
      <ul class="tips">{#each tips as t, i (i)}<li><span class="chip {t.tone}"><Icon name={t.tone === "bad" ? "alert" : t.tone === "ok" ? "sparkle" : "bolt"} /></span>{t.text}</li>{/each}</ul>
    </article>
  {/if}

  {#if !view.debts.length}
    <article class="card"><p class="muted">No tienes deudas. Si tienes una tarjeta de crédito, agrégala para llevar su límite, cortes y compras a meses.</p></article>
  {/if}

  {#if cards.length}
    {@render group("Tarjetas de crédito", sum(cards))}
    {#each cards as d (d.accountId)}
      {@const pct = Math.min(100, Math.round((d.utilization ?? 0) * 100))}
      <article class="card debt">
        <header class="debt-h">
          <button class="name" onclick={() => go(`cuenta-${d.accountId}`)}><AccountBadge account={look(d.accountId)} size={26} />{d.name}</button>
          {#if d.overdue}<span class="chip bad">Vencido · {date(d.dueDate!)}</span>
          {:else if d.dueDate && (d.toAvoidInterest ?? 0) > 0}<span class="chip warn">Pagar antes del {date(d.dueDate)}</span>
          {:else if d.dueDate}<span class="chip ok">Próximo pago {date(d.dueDate)}</span>{/if}
          <button class="btn icon sm ghost" aria-label="Datos de {d.name}" title="Datos de la tarjeta" onclick={() => openCard(d)}><Icon name="edit" /></button>
        </header>
        <div class="owed">
          <div><span class="muted">Debes</span> <b class="num">{money(d.debt)}</b></div>
          {#if d.terms?.creditLimit}<span class="muted num">de {money(d.terms.creditLimit)} de límite</span>{/if}
        </div>
        {#if d.terms?.creditLimit}
          <div class="bar" role="img" aria-label="Usas {pct}% del límite"><span style="width:{pct}%;--c:{pct > 30 ? 'var(--warn)' : 'var(--brand)'}"></span></div>
          <p class="muted small">{pct}% usado · disponible {money(d.available ?? 0)}</p>
        {/if}
        {#if d.statement !== null}
          {#if (d.toAvoidInterest ?? 0) > 0}
            <p class="callout" class:bad={d.overdue}>Paga <b class="num">{money(d.toAvoidInterest!)}</b> para no generar intereses{d.dueDate ? ` (antes del ${date(d.dueDate)})` : ""}.</p>
          {:else}<p class="callout ok"><Icon name="check" />Este corte ya está cubierto: no te cobrarán intereses.</p>{/if}
          <p class="muted small">
            Corte {date(d.lastCut!)} → {date(d.nextCut!)} · Saldo al corte {money(d.statement)}{#if d.minimum !== null}{` · Mínimo ${money(d.minimum)}`}{/if}{#if d.interestIfMinimum}{" · "}<span class="neg">interés si pagas el mínimo ~{money(d.interestIfMinimum)}</span>{/if}
          </p>
        {/if}
        {#if d.missing.length}
          <p class="note">Agrega {missingText(d)} y la app te dirá cuánto pagar y cómo evitar intereses. <button class="link" onclick={() => openCard(d)}>Completar datos</button></p>
        {/if}
        <div class="plans">
          <div class="plans-h"><h3>Compras a meses</h3><button class="btn sm ghost" onclick={() => openInstallment(d.accountId)}><Icon name="plus" />Agregar</button></div>
          {#each d.installments as p (p.installment.id)}
            <div class="row">
              <div class="grow"><p class="title"><button class="edit-name" title="Editar compra a meses" onclick={() => openInstallment(d.accountId, p.installment)}>{p.installment.name}</button></p>
                <p class="meta">{money(p.monthly)} al mes · {p.left ? `faltan ${p.left} de ${p.installment.months}` : "pagada"}{p.installment.annualRate ? ` · ${p.installment.annualRate / 100}% anual` : " · sin intereses"}</p>
                <div class="bar" style="margin-top:6px"><span style="width:{p.billed / p.installment.months * 100}%"></span></div></div>
              <b class="num">{money(p.remaining)}</b>
              <button class="btn icon sm ghost" aria-label="Editar {p.installment.name}" onclick={() => openInstallment(d.accountId, p.installment)}><Icon name="edit" /></button>
              <button class="btn icon sm ghost" aria-label="Quitar {p.installment.name}" onclick={() => remove(p.installment)}><Icon name="trash" /></button>
            </div>
          {:else}<p class="muted small">¿Compraste algo a meses? Agrégalo y la app lo descuenta del pago de cada corte.</p>{/each}
        </div>
      </article>
    {/each}
  {/if}

  {#if others.length}
    {@render group("Otras deudas", sum(others))}
    <article class="card">
      <div class="list">
        {#each others as d (d.accountId)}
          {@const card = looksLikeCard(d.name)}
          <div class="row">
            <AccountBadge account={look(d.accountId)} size={34} />
            <div class="grow"><button class="name" onclick={() => go(`cuenta-${d.accountId}`)}>{d.name}</button>
              <p class="meta">{card ? "Parece tarjeta de crédito" : "Saldo negativo"}</p></div>
            <b class="num neg">{money(-d.debt)}</b>
            {#if card}<button class="btn sm primary" onclick={() => openCard(d)}>Configurar tarjeta</button>
            {:else}<button class="btn sm ghost" onclick={() => openCard(d)}>¿Es tarjeta?</button>{/if}
          </div>
        {/each}
      </div>
    </article>
  {/if}
{/if}

<style>
  .tips { list-style: none; margin: 0; padding: 0; display: grid; gap: 10px; }
  .tips li { display: flex; gap: 10px; align-items: flex-start; line-height: 1.45; }
  .group { display: flex; justify-content: space-between; align-items: baseline; margin: 8px 4px -4px; }
  .group h2 { font-size: 13px; text-transform: uppercase; letter-spacing: .06em; color: var(--muted); margin: 0; }
  .name { all: unset; cursor: pointer; display: inline-flex; align-items: center; gap: 8px; font-weight: 600; }
  .name:hover { color: var(--brand); }
  .name:focus-visible { outline: 2px solid var(--brand); outline-offset: 2px; border-radius: 4px; }
  .debt { display: grid; gap: 10px; }
  .debt-h { display: flex; align-items: center; gap: 10px; }
  .debt-h .name { font-size: 16px; margin-right: auto; }
  .owed { display: flex; justify-content: space-between; align-items: baseline; gap: 10px; flex-wrap: wrap; }
  .owed b { font-size: 26px; }
  .small { font-size: 13px; margin: 0; }
  .callout { margin: 0; padding: 10px 12px; border-radius: 10px; background: var(--warn-soft); color: var(--warn); font-size: 14px; display: flex; gap: 6px; align-items: center; }
  .callout.bad { background: var(--expense-soft); color: var(--expense); }
  .callout.ok { background: var(--income-soft); color: var(--income); }
  .note { margin: 0; padding: 10px 12px; border-radius: 10px; background: var(--brand-soft); color: var(--brand); font-size: 13px; }
  .plans { display: grid; gap: 8px; border-top: 1px solid var(--border); padding-top: 10px; }
  .plans-h { display: flex; justify-content: space-between; align-items: center; }
  .plans h3 { font-size: 14px; margin: 0; }
</style>
