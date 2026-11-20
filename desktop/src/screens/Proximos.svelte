<script lang="ts">
  import * as api from "../lib/api";
  import { app, refresh } from "../lib/app.svelte";
  import { openSchedule } from "../lib/dialogs.svelte";
  import { categoryColor, daysBetween, dayMonth, dayNumber, money, monthKey, parseDay } from "../lib/format";
  import { remember, toast } from "../lib/undo.svelte";
  import Icon from "../components/Icon.svelte";
  import Kpi from "../components/Kpi.svelte";
  import Topbar from "../components/Topbar.svelte";

  let items = $state<api.UpcomingItem[]>([]);
  let schedules = $state<api.ScheduleView[]>([]);
  let error = $state("");
  let busy = $state(false);

  $effect(() => {
    void app.version;
    Promise.all([api.upcoming(), api.schedules()]).then(([u, s]) => { items = u; schedules = s; error = ""; }).catch(e => error = String(e));
  });

  const days = (u: api.UpcomingItem) => daysBetween(app.today, parseDay(u.date));
  let overdue = $derived(items.filter(u => u.overdue));
  let week = $derived(items.filter(u => !u.overdue && days(u) <= 7));
  let later = $derived(items.filter(u => !u.overdue && days(u) > 7));
  // Only the earliest occurrence of a schedule can be registered or skipped: those act on "the next one".
  let actionable = $derived.by(() => { const seen = new Set<number>(); return new Set(items.filter(u => !seen.has(u.scheduleId) && seen.add(u.scheduleId)).map(u => `${u.scheduleId}:${u.date}`)); });
  const sum = (list: api.UpcomingItem[]) => list.reduce((s, u) => s + u.amount, 0);
  let total = $derived(app.accounts.filter(a => !a.closed).reduce((s, a) => s + a.balance, 0));
  let endOfMonth = $derived(total + sum(items.filter(u => monthKey(parseDay(u.date)) === monthKey(app.today))));

  async function act(label: string, run: () => Promise<() => Promise<unknown>>) {
    if (busy) return;
    busy = true;
    try { const revert = await run(); remember(label, () => revert().then(refresh)); await refresh(); }
    catch (e) { toast(String(e)); } finally { busy = false; }
  }
  const register = (u: api.UpcomingItem) => act(`Registrado: ${u.name} ${money(u.amount, true)}`, async () => {
    const { entry, before } = await api.postSchedule(u.scheduleId);
    return async () => { await api.deleteEntry(entry.id); await api.restoreSchedule(before); };
  });
  const skip = (u: api.UpcomingItem) => act(`Saltaste ${u.name} del ${dayNumber(parseDay(u.date))} ${dayMonth(parseDay(u.date))}`, async () => {
    const before = await api.skipSchedule(u.scheduleId);
    return () => api.restoreSchedule(before);
  });
  const remove = (v: api.ScheduleView) => act(`Eliminaste “${v.schedule.name}”`, async () => {
    const before = await api.deleteSchedule(v.schedule.id);
    return () => api.restoreSchedule(before);
  });
</script>

{#snippet row(u: api.UpcomingItem)}
  {@const d = parseDay(u.date)}{@const n = days(u)}{@const canAct = actionable.has(`${u.scheduleId}:${u.date}`)}
  <div class="row">
    <div class="date" class:over={u.overdue}><b>{dayNumber(d)}</b><small>{dayMonth(d)}</small></div>
    <div class="grow">
      <p class="title">{u.name}</p>
      <p class="meta">{u.account ?? "Sin cuenta"} · {#if u.overdue}<span class="chip bad">Vencido hace {-n} d</span>{:else if n === 0}<span class="chip warn">Hoy</span>{:else}en {n} días{/if}{#if u.autoPost}{" · se registra solo"}{/if}</p>
    </div>
    <div style="display:grid;justify-items:end;gap:2px">
      <b class="num" class:pos={u.amount > 0}>{money(u.amount, true)}</b>
      {#if u.projectedBalance !== null}<small class="muted num" class:neg={u.projectedBalance < 0}>Saldo: {money(u.projectedBalance)}</small>{/if}
    </div>
    {#if canAct}
      <button class="btn sm primary" disabled={busy} onclick={() => register(u)}><Icon name="check" />Registrar</button>
      <button class="btn sm ghost" disabled={busy} onclick={() => skip(u)}>Saltar</button>
    {:else}<span class="muted" style="font-size:12px;width:150px;text-align:right">Después del anterior</span>{/if}
  </div>
{/snippet}

{#snippet section(title: string, list: api.UpcomingItem[], empty: string)}
  <article class="card">
    <div class="card-h"><h2>{title}</h2><span class="chip">{list.length}</span></div>
    {#if list.length}<div class="list">{#each list as u (u.scheduleId + u.date)}{@render row(u)}{/each}</div>{:else}<p class="muted">{empty}</p>{/if}
  </article>
{/snippet}

<Topbar title="Próximos pagos" sub="Anticipa tus compromisos">
  {#snippet actions()}<button class="btn primary" onclick={() => openSchedule()}><Icon name="plus" />Programar pago</button>{/snippet}
</Topbar>

{#if error}<p class="err" role="alert">{error}</p>{/if}
<section class="kpis">
  <Kpi icon="alert" tone="bad" label="Vencido" cents={-sum(overdue.filter(u => u.amount < 0))} note="{overdue.length} pago{overdue.length === 1 ? '' : 's'}" />
  <Kpi icon="clock" tone="warn" label="Por pagar en 30 días" cents={-sum(items.filter(u => u.amount < 0))} />
  <Kpi icon="wallet" tone="ok" label="Saldo proyectado a fin de mes" cents={endOfMonth} />
</section>
{#if overdue.length}{@render section("Vencidos", overdue, "")}{/if}
{@render section("Esta semana", week, "Nada esta semana.")}
{@render section("Más adelante", later, "Nada más en los próximos 30 días.")}

<article class="card">
  <div class="card-h"><h2>Pagos programados</h2>{#if app.settings?.enabled}<span class="chip brand"><Icon name="bell" />Te aviso {app.settings.days} días antes</span>{/if}</div>
  <div class="list">
    {#each schedules as v (v.schedule.id)}
      <div class="row">
        <span class="merchant" style="--c:{categoryColor(v.schedule.categoryId)}"><Icon name="repeat" /></span>
        <div class="grow"><p class="title">{v.schedule.name}</p><p class="meta">{v.label}{v.account ? ` · ${v.account}` : ""}</p></div>
        <b class="num" class:pos={v.schedule.amount > 0}>{money(v.schedule.amount, true)}</b>
        <button class="btn icon sm ghost" aria-label="Editar {v.schedule.name}" onclick={() => openSchedule(v)}><Icon name="edit" /></button>
        <button class="btn icon sm ghost" aria-label="Eliminar {v.schedule.name}" disabled={busy} onclick={() => remove(v)}><Icon name="trash" /></button>
      </div>
    {:else}<p class="muted">Aún no programas pagos. La renta, servicios o tu sueldo son buenos candidatos.</p>{/each}
  </div>
</article>
