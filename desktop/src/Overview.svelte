<script lang="ts">
  import { overview, formatMoney, formatDate, type Overview } from './lib/api';
  import MonthNav from './MonthNav.svelte';
  import Imports from './Imports.svelte';
  let { version, month = $bindable(), currentMonth, refresh }: {version: string; month: number; currentMonth: number; refresh: () => void} = $props();
  let range = $state(6);
  let data = $state<Overview | null>(null); let error = $state(''); let request = 0;
  const percent = (v: number) => new Intl.NumberFormat('es-MX', {style:'percent', maximumFractionDigits:1}).format(v);
  const title = (m: number, short = false) => {
    const date = new Date(Math.floor(m / 100), m % 100 - 1, 1);
    const text = new Intl.DateTimeFormat('es-MX', short ? {month:'short'} : {month:'long', year:'numeric'}).format(date).replace('.', '');
    return text.charAt(0).toUpperCase() + text.slice(1);
  };

  $effect(() => { const m = month, r = range; void version; if (!m) return; const id = ++request; void overview(m, r).then(v => { if (id === request) { data = v; error = ''; } }).catch(e => { if (id === request) error = String(e); }); });
</script>
<div class="screen">
  <div class="heading"><div><h1>Tu panorama financiero</h1><p>{month ? title(month) : ''}</p></div><MonthNav bind:month {currentMonth}/></div>
  {#if error}<p role="alert" class="error">{error}</p>{/if}
  <Imports {version} firstRun {refresh}/>
  {#if data}
    {@const max = Math.max(1, ...data.chart.flatMap(p => [p.income, p.expenses]))}
    {@const ratio = data.budgetAssigned > 0 ? data.budgetSpent / data.budgetAssigned : 0}
    {#if !data.firstRun.hasAccount || !data.firstRun.hasCategorizedIncome || !data.firstRun.hasBudget}
      <section class="card"><h2>Primeros pasos</h2><div class="steps">{#each [[data.firstRun.hasAccount, 'Crea una cuenta', 'En la barra lateral, «Nueva cuenta».'], [data.firstRun.hasCategorizedIncome, 'Registra tu ingreso', 'Un movimiento tipo Ingreso con categoría de ingreso.'], [data.firstRun.hasBudget, 'Asígnalo en el presupuesto', 'En Presupuesto, escribe cuánto va a cada categoría.']] as step}<div><span class="check">{step[0] ? '✓' : '○'}</span><span><strong>{step[1]}</strong><small>{step[2]}</small></span></div>{/each}</div></section>
    {/if}
    <div class="stats">
      <section class="card hero"><h2>Saldo total</h2><strong>{formatMoney(data.totalBalance)}</strong><p>{data.openAccountCount} {data.openAccountCount === 1 ? 'cuenta abierta' : 'cuentas abiertas'}</p></section>
      <section class="card"><h2>Ingresos del mes</h2><strong>{formatMoney(data.current.income)}</strong><p>{data.incomeChange === null ? 'Sin datos del mes anterior' : `${data.incomeChange >= 0 ? '+' : ''}${percent(data.incomeChange)} vs mes anterior`}</p></section>
      <section class="card"><h2>Gastos del mes</h2><strong>{formatMoney(data.current.expenses)}</strong><p>{data.expensesChange === null ? 'Sin datos del mes anterior' : `${data.expensesChange >= 0 ? '+' : ''}${percent(data.expensesChange)} vs mes anterior`}</p></section>
      <section class="card"><h2>Tasa de ahorro</h2><strong>{data.current.savingsRate === null ? '—' : percent(data.current.savingsRate)}</strong><p>{data.current.savingsRate === null ? 'Sin ingresos este mes' : `${formatMoney(data.current.income - data.current.expenses)} ahorrados`}</p></section>
    </div>
    <div class="grid"><section class="card"><div class="heading"><h2>Ingresos vs gastos</h2><div class="actions"><button class:chosen={range === 6} onclick={() => range = 6}>6M</button><button class:chosen={range === 12} onclick={() => range = 12}>12M</button></div></div>
      <svg viewBox="0 0 600 240" role="img" aria-label="Ingresos y gastos por mes"><line x1="0" y1="200" x2="600" y2="200" stroke="var(--border)"/>{#each data.chart as point, i}{@const width = 600 / data.chart.length}<rect x={i * width + width * .15} y={200 - Math.max(0, point.income) / max * 180} width={width * .3} height={Math.max(0, point.income) / max * 180} fill="var(--emerald)"/><rect x={i * width + width * .5} y={200 - Math.max(0, point.expenses) / max * 180} width={width * .3} height={Math.max(0, point.expenses) / max * 180} fill="var(--coral)"/><text x={i * width + width / 2} y="222" text-anchor="middle" fill="currentColor" font-size="12">{title(point.month, true)}</text>{/each}</svg><p>Flujo neto promedio: <b>{formatMoney(data.chart.reduce((s, p) => s + p.income - p.expenses, 0) / (data.chart.length || 1))}</b></p></section>
      <section class="card"><h2>Gasto por categoría</h2><p>Total: {formatMoney(data.spendingTotal)}</p>{#each data.spending as item}<div class="row"><span>{item.name}</span><b>{formatMoney(item.spent)}</b><span>{percent(data.spendingTotal ? item.spent / data.spendingTotal : 0)}</span></div><progress max="1" value={data.spendingTotal ? item.spent / data.spendingTotal : 0}></progress>{:else}<p>Sin gastos este mes</p>{/each}</section></div>
    <div class="grid"><section class="card tablewrap"><h2>Movimientos recientes</h2><table><thead><tr><th>Descripción</th><th>Categoría</th><th>Cuenta</th><th>Fecha</th><th>Monto</th></tr></thead><tbody>{#each data.recentEntries as entry}<tr><td>{entry.payeeName || entry.notes || '—'}</td><td>{entry.categoryName || 'Sin categoría'}</td><td>{entry.accountName || ''}</td><td>{formatDate(entry.date)}</td><td class:expense={entry.amount < 0} class:income={entry.amount >= 0}>{formatMoney(entry.amount)}</td></tr>{/each}</tbody></table></section>
      <section class="card"><h2>Presupuesto del mes</h2><div class="row"><strong>{formatMoney(data.budgetSpent)}</strong><span>de {formatMoney(data.budgetAssigned)}</span></div><progress class:warning={ratio >= .9} class:over={ratio > 1} max="1" value={Math.min(ratio, 1)}></progress><div class="row"><span>{data.budgetAssigned > 0 ? `${percent(ratio)} usado` : 'Sin asignar'}</span>{#if data.budgetAssigned > 0}<span>{formatMoney(Math.abs(data.budgetAssigned - data.budgetSpent))} {ratio > 1 ? 'excedido' : 'restante'}</span>{/if}</div>{#each data.budgetProgress as item}<div class="row"><span>{item.name}</span><span>{formatMoney(item.spent)} / {formatMoney(item.budgeted)}</span></div>{/each}</section></div>
  {/if}
</div>
<style>
.screen{display:grid;gap:20px;max-width:1500px;margin:auto}.heading,.row,.actions{display:flex;align-items:center;justify-content:space-between;gap:10px}.actions{justify-content:flex-start;flex-wrap:wrap}h1,h2,p{margin:0}h2{font-size:1rem;margin-bottom:12px}p,small{color:var(--muted);font-size:.9rem}small{display:block}.card{background:var(--card);border:1px solid var(--border);border-radius:18px;padding:20px;min-width:0}.hero{background:var(--hero);color:white}.hero p{color:#e4f3f5}.stats{display:grid;grid-template-columns:repeat(auto-fit,minmax(190px,1fr));gap:16px}.stats strong,.row strong{font-size:1.6rem}.stats p{margin-top:14px}.grid{display:grid;grid-template-columns:minmax(0,2fr) minmax(260px,1fr);gap:16px}.steps{display:grid;gap:12px}.steps>div{display:flex;gap:12px}.check,.income{color:var(--emerald)}.expense,.error{color:var(--coral)}button{font:inherit;color:inherit;background:var(--canvas);border:1px solid var(--border);border-radius:8px;padding:7px;cursor:pointer}.chosen{color:var(--petroleum);font-weight:bold}svg{width:100%;height:240px}progress{width:100%;height:8px;accent-color:var(--petroleum)}progress.warning{accent-color:var(--amber)}progress.over{accent-color:var(--coral)}.row{margin:12px 0;font-size:.9rem}.tablewrap{overflow:auto}table{border-collapse:collapse;width:100%;font-size:.85rem}th,td{text-align:left;padding:8px;white-space:nowrap;border-bottom:1px solid var(--border)}@media(max-width:850px){.grid{grid-template-columns:1fr}.heading{flex-wrap:wrap}}
</style>
