<script lang="ts">
 import { assignBudget, formatMoney, type BudgetRow } from './lib/api';
 let {row, month, refresh}: {row:BudgetRow; month:number; refresh:()=>void} = $props();
 let text = $state(''); let invalid = $state(false); let busy = $state(false); let focused = $state(false); let last = $state('');
 $effect(() => { const formatted = formatMoney(row.budgeted); if (!focused && !invalid && !busy) { text = formatted; last = formatted; } });
 async function commit() { if (busy || text === last) return; busy = true; try { const value = await assignBudget(month, row.id, text); if (value === null) invalid = true; else { invalid = false; text = formatMoney(value); last = text; refresh(); } } catch { invalid = true; } finally { busy = false; } }
</script>
<td><div>{row.name}</div><progress max="1" value={row.spentShare} aria-label={`Gastado ${Math.round(row.spentShare*100)}%`}></progress></td>
<td><input aria-label={`Asignado a ${row.name}`} aria-invalid={invalid} bind:value={text} onfocus={() => focused = true} onblur={() => { focused = false; void commit(); }} onkeydown={e => {if(e.key === 'Enter') {e.preventDefault(); void commit();}}}/>{#if invalid}<small role="alert">Monto inválido</small>{/if}</td><td>{formatMoney(row.activity)}</td><td class:negative={row.available < 0}>{formatMoney(row.available)}</td>
<style>td{padding:10px;border-bottom:1px solid var(--border)}td:not(:first-child){text-align:right}input{width:110px;text-align:right;font:inherit;padding:7px;background:var(--canvas);color:inherit;border:1px solid var(--border);border-radius:6px}input[aria-invalid=true]{border-color:var(--coral)}small,.negative{color:var(--coral)}small{display:block}progress{display:block;width:min(100%,280px);height:6px;accent-color:var(--petroleum);margin-top:5px}</style>
