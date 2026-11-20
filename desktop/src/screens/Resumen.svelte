<script lang="ts">
  import * as api from "../lib/api";
  import { app, go } from "../lib/app.svelte";
  import { openAccount, openEntry } from "../lib/dialogs.svelte";
  import { money, monthKey, monthTitle, monthShort, shiftMonth, change, categoryColor, parseDay, daysBetween, dayNumber, dayMonth, monthDate } from "../lib/format";
  import Icon from "../components/Icon.svelte";
  import Kpi from "../components/Kpi.svelte";
  import Money from "../components/Money.svelte";
  import Topbar from "../components/Topbar.svelte";

  let month = $state(monthKey(app.today));
  let range = $state(6);
  let data = $state<{ overview: api.Overview; upcoming: api.UpcomingItem[]; reserved: number } | null>(null);
  let error = $state("");

  $effect(() => {
    void app.version; const m = month, r = range;
    Promise.all([api.overview(m, r), api.upcoming(), api.wishes()])
      .then(([overview, upcoming, wishes]) => { data = { overview, upcoming, reserved: wishes.reduce((s, w) => s + w.wish.reserved, 0) }; error = ""; })
      .catch(e => error = String(e));
  });

  let o = $derived(data?.overview);
  let income = $derived(o?.current.income ?? 0);
  let spent = $derived(o?.current.expenses ?? 0);
  // Still to pay this month: scheduled outflows dated in the shown month.
  let pending = $derived(data ? -data.upcoming.filter(u => u.amount < 0 && monthKey(parseDay(u.date)) === month).reduce((s, u) => s + u.amount, 0) : 0);
  let free = $derived(income - spent - pending);
  const pct = (n: number) => income > 0 ? Math.max(0, Math.min(100, n / income * 100)) : 0;
  let stops = $derived.by(() => {
    if (!o || !o.spendingTotal) return "var(--surface-2) 0 100%";
    let at = 0;
    return o.spending.map(c => { const from = at; at += c.spent / o.spendingTotal * 100; return `${categoryColor(c.categoryId)} ${from}% ${at}%`; }).join(",");
  });
  let maxBar = $derived(o ? Math.max(1, ...o.chart.map(p => Math.max(p.income, p.expenses))) * 1.08 : 1);
  let average = $derived(o && o.chart.length ? o.chart.reduce((s, p) => s + p.income - p.expenses, 0) / o.chart.length : 0);
</script>

<Topbar title={monthTitle(month)} sub="Resumen">
  {#snippet lead()}
    <div class="month">
      <button class="btn icon" aria-label="Mes anterior" onclick={() => month = shiftMonth(month, -1)}><Icon name="left" /></button>
      <button class="btn icon" aria-label="Mes siguiente" onclick={() => month = shiftMonth(month, 1)}><Icon name="right" /></button>
    </div>
  {/snippet}
  {#snippet actions()}
    <button class="btn" onclick={() => openEntry("ingreso")}><Icon name="plus" />Ingreso</button>
    <button class="btn primary" onclick={() => openEntry("gasto")}><Icon name="minus" />Gasto</button>
  {/snippet}
</Topbar>

{#if error}<p class="err" role="alert">{error}</p>{/if}
{#if o && data}
  {#if !o.firstRun.hasAccount || !o.firstRun.hasCategorizedIncome || !o.firstRun.hasBudget}
    <article class="card">
      <div class="card-h"><h2>Primeros pasos</h2></div>
      <div class="list">
        {#each [[o.firstRun.hasAccount, "Crea una cuenta", "Tu banco, efectivo o tarjeta.", "Nueva cuenta", openAccount], [o.firstRun.hasCategorizedIncome, "Registra un ingreso con categoría", "Así el presupuesto sabe cuánto puedes asignar.", "Nuevo ingreso", () => openEntry("ingreso")], [o.firstRun.hasBudget, "Asigna tu presupuesto", "Dale un destino a cada peso.", "Ir a Presupuesto", () => go("presupuesto")]] as [done, title, text, label, run] (title)}
          <div class="row">
            <span class="ic sm {done ? 'ok' : ''}"><Icon name={done ? "check" : "right"} /></span>
            <div class="grow"><p class="title" style:text-decoration={done ? "line-through" : "none"}>{title}</p><p class="meta">{text}</p></div>
            {#if !done}<button class="btn sm primary" onclick={run as () => void}>{label}</button>{/if}
          </div>
        {/each}
      </div>
    </article>
  {/if}
  <section class="grid g-3">
    <article class="card hero">
      <p class="label">{free >= 0 ? "Te sobra este mes" : "Te falta este mes"}</p>
      <Money cents={free} />
      <p class="note">Después de pagar lo programado · {pct(free).toFixed(0)}% de tus ingresos</p>
      <div class="flow" role="img" aria-label="Gastado {pct(spent).toFixed(0)}%, por pagar {pct(pending).toFixed(0)}%, libre {pct(free).toFixed(0)}%">
        <span style="width:{pct(spent)}%;background:#fff"></span><span style="width:{pct(pending)}%;background:rgb(255 255 255 / .55)"></span><span style="width:{pct(free)}%;background:#7CF0C0"></span>
      </div>
      <div class="legend">
        <div><p><i style="background:rgb(255 255 255 / .3)"></i>Ingresos</p><b>{money(income)}</b></div>
        <div><p><i style="background:#fff"></i>Gastado</p><b>{money(spent)}</b></div>
        <div><p><i style="background:rgb(255 255 255 / .55)"></i>Por pagar</p><b>{money(pending)}</b></div>
        <div><p><i style="background:#7CF0C0"></i>Libre</p><b>{money(free)}</b></div>
      </div>
    </article>
    <article class="card">
      <div class="card-h"><h2>Pagos por hacer</h2><button class="link" onclick={() => go("proximos")}>Ver todos</button></div>
      <div class="list">
        {#each data.upcoming.slice(0, 3) as u (u.scheduleId + u.date)}
          {@const d = parseDay(u.date)}{@const days = daysBetween(app.today, d)}
          <div class="row">
            <div class="date" class:over={u.overdue}><b>{dayNumber(d)}</b><small>{dayMonth(d)}</small></div>
            <div class="grow"><p class="title">{u.name}</p>
              <p class="meta">{#if u.overdue}<span class="chip bad">Vencido hace {-days} d</span>{:else if days === 0}<span class="chip warn">Hoy</span>{:else}en {days} días{/if}{#if u.autoPost} · automático{/if}</p></div>
            <b class="num" class:pos={u.amount > 0}>{money(u.amount, true)}</b>
          </div>
        {:else}<p class="muted">Nada programado en los próximos 30 días.</p>{/each}
      </div>
    </article>
  </section>

  <section class="kpis">
    <Kpi icon="wallet" label="Saldo en cuentas" cents={o.totalBalance} />
    <Kpi icon="up" tone="ok" label="Ingresos" cents={income} note={change(o.incomeChange, shiftMonth(month, -1))} />
    <Kpi icon="down" tone="bad" label="Gastos" cents={spent} note={change(o.expensesChange, shiftMonth(month, -1))} />
    <Kpi icon="gift" tone="violet" label="Apartado en deseos" cents={data.reserved} />
  </section>

  <section class="grid g-2">
    <article class="card">
      <div class="card-h"><h2>¿En qué se va el dinero?</h2><button class="link" onclick={() => go(`movimientos?mes=${month}`)}>Movimientos</button></div>
      {#if o.spending.length}
        <div style="display:flex;gap:22px;align-items:center;flex-wrap:wrap">
          <div class="donut" style="background:conic-gradient({stops})" role="img" aria-label="Gasto por categoría"><div><span><b class="num">{money(o.spendingTotal)}</b><small>gastado</small></span></div></div>
          <div class="cats">
            {#each o.spending as c (c.categoryId)}
              <button class="cat" style="--c:{categoryColor(c.categoryId)}" onclick={() => go(`movimientos?mes=${month}&categoria=${c.categoryId}`)}>
                <i></i><span>{c.name}</span><span class="muted num">{(c.spent / o.spendingTotal * 100).toFixed(0)}%</span><b class="num">{money(c.spent)}</b>
              </button>
            {/each}
          </div>
        </div>
      {:else}<p class="muted">Sin gastos con categoría este mes.</p>{/if}
    </article>
    <article class="card">
      <div class="card-h"><h2>Ingresos vs gastos</h2>
        <div class="seg" style="margin-left:auto" role="group" aria-label="Rango">
          <button aria-pressed={range === 6} onclick={() => range = 6}>6 meses</button><button aria-pressed={range === 12} onclick={() => range = 12}>12 meses</button>
        </div>
      </div>
      <div class="chart" class:dense={range > 6}>
        {#each o.chart as p (p.month)}
          {@const name = monthDate(p.month).toLocaleDateString("es-MX", { month: "long", year: "numeric" })}
          <button class="col" class:cur={p.month === month} onclick={() => go(`movimientos?mes=${p.month}`)} aria-label="{name}: ingresos {money(p.income)}, gastos {money(p.expenses)}. Ver movimientos">
            <span class="pair"><span style="height:{p.income / maxBar * 100}%;background:var(--income)"></span><span style="height:{p.expenses / maxBar * 100}%;background:var(--expense)"></span></span>
            <small>{monthShort(p.month)}</small>
            <span class="tip" aria-hidden="true"><b style="text-transform:capitalize">{name}</b><span>Ingresos <b>{money(p.income)}</b></span><span>Gastos <b>{money(p.expenses)}</b></span><span>Neto <b>{money(p.income - p.expenses, true)}</b></span><small style="opacity:.7">Clic para ver movimientos</small></span>
          </button>
        {/each}
      </div>
      <div style="display:flex;justify-content:space-between;margin-top:14px;padding-top:14px;border-top:1px solid var(--border)"><span class="muted">Ahorro promedio al mes</span><b class="num" class:pos={average >= 0} class:neg={average < 0}>{money(Math.round(average))}</b></div>
    </article>
  </section>
{/if}
