<script lang="ts">
  import * as api from "../lib/api";
  import { app, go, refresh } from "../lib/app.svelte";
  import { openCard, openInstallment } from "../lib/dialogs.svelte";
  import { advice, missingText } from "../lib/debts";
  import { money, parseDay } from "../lib/format";
  import { remember, toast } from "../lib/undo.svelte";
  import Icon from "../components/Icon.svelte";
  import Kpi from "../components/Kpi.svelte";
  import Topbar from "../components/Topbar.svelte";

  let view = $state<api.DebtsView | null>(null);
  let error = $state("");
  $effect(() => { void app.version; api.debts().then(v => { view = v; error = ""; }).catch(e => error = String(e)); });
  let tips = $derived(view ? advice(view.debts) : []);
  let toPay = $derived(view?.debts.reduce((s, d) => s + (d.toAvoidInterest ?? 0), 0) ?? 0);
  const date = (iso: string) => parseDay(iso).toLocaleDateString("es-MX", { day: "numeric", month: "short" });

  async function remove(p: api.Installment) {
    try {
      const removed = await api.deleteInstallment(p.id);
      remember(`Quitaste “${removed.name}”`, () => api.saveInstallment(null, removed).then(refresh));
      await refresh();
    } catch (e) { toast(String(e)); }
  }
</script>

{#snippet fact(label: string, value: string, tone = "")}
  <div class="fact"><span class="muted">{label}</span><b class="num {tone}">{value}</b></div>
{/snippet}

<Topbar title="Deudas" sub="Todo lo que debes, en un lugar">
  {#snippet actions()}<button class="btn primary" onclick={() => openCard()}><Icon name="plus" />Agregar tarjeta</button>{/snippet}
</Topbar>

{#if error}<p class="err" role="alert">{error}</p>{/if}
{#if view}
  <section class="kpis">
    <Kpi icon="card" tone="bad" label="Debes en total" cents={view.total} note="{view.debts.length} deuda{view.debts.length === 1 ? '' : 's'}" />
    <Kpi icon="clock" tone="warn" label="Para no pagar intereses" cents={toPay} note="Suma de este corte" />
    <Kpi icon="repeat" tone="ok" label="Mensualidades a meses" cents={view.monthlyInstallments} />
  </section>

  {#if tips.length}
    <article class="card">
      <div class="card-h"><h2>Qué te recomiendo</h2></div>
      <ul class="tips">{#each tips as t, i (i)}<li><span class="chip {t.tone}"><Icon name={t.tone === "bad" ? "alert" : t.tone === "ok" ? "sparkle" : "bolt"} /></span>{t.text}</li>{/each}</ul>
    </article>
  {/if}

  {#each view.debts as d (d.accountId)}
    <article class="card debt">
      <div class="card-h">
        <h2><button class="link" onclick={() => go(`cuenta-${d.accountId}`)}>{d.name}</button></h2>
        <span class="chip">{d.isCard ? "Tarjeta de crédito" : "Préstamo o saldo negativo"}</span>
        {#if d.overdue}<span class="chip bad">Vencido</span>{/if}
        <button class="btn sm ghost" style="margin-left:auto" onclick={() => openCard(d)}><Icon name="edit" />{d.isCard ? "Datos de la tarjeta" : "Es tarjeta de crédito"}</button>
      </div>
      <div class="owed"><span class="muted">Debes</span><b class="num">{money(d.debt)}</b></div>
      {#if d.terms?.creditLimit}
        {@const pct = Math.min(100, Math.round((d.utilization ?? 0) * 100))}
        <div class="bar" role="img" aria-label="Usas {pct}% del límite"><span style="width:{pct}%;--c:{pct > 30 ? 'var(--warn)' : 'var(--brand)'}"></span></div>
        <p class="muted small">Disponible {money(d.available ?? 0)} de {money(d.terms.creditLimit)} · usas {pct}%</p>
      {/if}
      {#if d.statement !== null}
        <div class="facts">
          {@render fact("Corte", `${date(d.lastCut!)} → ${date(d.nextCut!)}`)}
          {#if d.dueDate}{@render fact("Pagar antes del", date(d.dueDate), d.overdue ? "neg" : "")}{/if}
          {@render fact("Saldo al corte", money(d.statement))}
          {@render fact("Para no generar intereses", money(d.toAvoidInterest ?? 0), (d.toAvoidInterest ?? 0) > 0 ? "neg" : "pos")}
          {#if d.minimum !== null}{@render fact("Pago mínimo", money(d.minimum))}{/if}
          {#if d.interestIfMinimum}{@render fact("Interés si pagas el mínimo", `~${money(d.interestIfMinimum)}`, "neg")}{/if}
        </div>
      {/if}
      {#if d.missing.length}
        <p class="card-note">Agrega {missingText(d)} y la app te dirá cuánto pagar y cómo evitar intereses. <button class="link" onclick={() => openCard(d)}>Completar datos</button></p>
      {/if}
      {#if d.isCard}
        <div class="plans">
          <div class="card-h"><h3>Compras a meses</h3><button class="btn sm" onclick={() => openInstallment(d.accountId)}><Icon name="plus" />Agregar</button></div>
          {#each d.installments as p (p.installment.id)}
            <div class="row">
              <div class="grow"><p class="title">{p.installment.name}</p>
                <p class="meta">{money(p.monthly)} al mes · {p.left ? `faltan ${p.left} de ${p.installment.months}` : "pagada"}{p.installment.annualRate ? ` · ${p.installment.annualRate / 100}% anual` : " · sin intereses"}</p>
                <div class="bar" style="margin-top:6px"><span style="width:{p.billed / p.installment.months * 100}%"></span></div></div>
              <b class="num">{money(p.remaining)}</b>
              <button class="btn icon sm ghost" aria-label="Editar {p.installment.name}" onclick={() => openInstallment(d.accountId, p.installment)}><Icon name="edit" /></button>
              <button class="btn icon sm ghost" aria-label="Quitar {p.installment.name}" onclick={() => remove(p.installment)}><Icon name="trash" /></button>
            </div>
          {:else}<p class="muted small">¿Compraste algo a meses sin intereses? Agrégalo y la app lo descuenta del pago de cada corte.</p>{/each}
        </div>
      {/if}
    </article>
  {:else}
    <article class="card"><p class="muted">No tienes deudas. Si tienes una tarjeta de crédito, agrégala para llevar su límite, cortes y compras a meses.</p></article>
  {/each}
{/if}

<style>
  .tips { list-style: none; margin: 0; padding: 0; display: grid; gap: 10px; }
  .tips li { display: flex; gap: 10px; align-items: flex-start; line-height: 1.45; }
  .debt { display: grid; gap: 12px; }
  .owed { display: flex; align-items: baseline; gap: 10px; }
  .owed b { font-size: 28px; }
  .small { font-size: 13px; margin: 0; }
  .facts { display: grid; grid-template-columns: repeat(auto-fit, minmax(170px, 1fr)); gap: 10px; }
  .fact { display: grid; gap: 2px; padding: 10px 12px; border-radius: 10px; background: var(--surface-2); font-size: 13px; }
  .fact b { font-size: 15px; }
  .card-note { margin: 0; padding: 10px 12px; border-radius: 10px; background: var(--brand-soft); color: var(--brand); font-size: 13px; }
  .plans { display: grid; gap: 8px; border-top: 1px solid var(--border); padding-top: 12px; }
  .plans h3 { font-size: 14px; margin: 0; }
</style>
