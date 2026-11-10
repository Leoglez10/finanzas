<script lang="ts">
  import { onMount, tick, untrack } from 'svelte';
  import { addEntry, updateEntry, parseAmount, payeeSuggestions, suggestedCategory, categoriesGrouped, today, type AccountOverview, type EntryRow, type CategoryGroup } from './lib/api';
  let { accounts, accountId, editing, startExpense, close, saved }: { accounts: AccountOverview[]; accountId?: number; editing: EntryRow | null; startExpense: boolean; close: () => void; saved: () => void } = $props();
  const initial = untrack(() => ({editing, startExpense, accountId, firstAccount: accounts[0]?.id}));
  let expense = $state(initial.editing ? initial.editing.amount <= 0 : initial.startExpense);
  let amount = $state(initial.editing ? (Math.abs(initial.editing.amount) / 100).toFixed(2) : '');
  let account = $state<string>(String(initial.editing?.accountId ?? initial.accountId ?? initial.firstAccount ?? ''));
  let payee = $state(initial.editing?.payeeName ?? ''); let category = $state<string>(initial.editing?.categoryId?.toString() ?? '');
  let date = $state(initial.editing?.date ?? ''); let notes = $state(initial.editing?.notes ?? '');
  let groups = $state<CategoryGroup[]>([]); let suggestions = $state<string[]>([]); let cents = $state<number | null>(null); let error = $state(''); let saving = $state(false); let dialog: HTMLDivElement;
  let parseRequest = 0, payeeRequest = 0;
  onMount(() => { void Promise.all([categoriesGrouped(), today()]).then(([g, d]) => { groups = g; if (!date) date = d; if (!editing && !expense) selectIncome(); }).catch(e => error = String(e)); void tick().then(() => dialog?.querySelector<HTMLInputElement>('input')?.focus()); });
  $effect(() => { const text = amount; const id = ++parseRequest; cents = null; void parseAmount(text).then(v => { if (id === parseRequest) cents = v; }).catch(e => error = String(e)); });
  $effect(() => { const text = payee; const id = ++payeeRequest; if (!text.trim()) { suggestions = []; return; } void payeeSuggestions(text).then(v => { if (id === payeeRequest) suggestions = v; }).catch(e => error = String(e)); });
  function selectIncome() { category = groups.filter(g => g.isIncome).flatMap(g => g.categories).find(c => !c.hidden)?.id.toString() ?? ''; }
  function setType(next: boolean) { if (expense === next) return; expense = next; if (expense) category = ''; else selectIncome(); }
  async function prefill(name: string) { try { const id = await suggestedCategory(name); if (name === payee && id !== null && groups.some(g => g.categories.some(c => c.id === id && !c.hidden))) category = String(id); } catch (e) { error = String(e); } }
  async function save() { if (cents === null || !account || !date || saving) return; saving = true; try { const entry = {date, amount: expense ? -cents : cents, accountId: Number(account), payee, categoryId: category ? Number(category) : null, notes}; if (editing) await updateEntry(editing.id, entry); else await addEntry(entry); saved(); } catch (e) { error = String(e); saving = false; } }
  function keys(e: KeyboardEvent) { if (e.key === 'Escape') { e.preventDefault(); close(); } else if (e.key === 'Tab') { const els = Array.from(dialog.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled)')); if (!els.length) return; const first = els[0], last = els[els.length - 1]; if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); } else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); } } }
</script>
<div class="backdrop"><div class="modal" role="dialog" tabindex="-1" aria-modal="true" aria-labelledby="sheet-title" bind:this={dialog} onkeydown={keys}>
  <h2 id="sheet-title">{editing ? 'Editar movimiento' : 'Nuevo movimiento'}</h2><p>Registra los detalles de tu ingreso o gasto</p>
  <form onsubmit={(e) => {e.preventDefault(); void save();}}>
    <fieldset><legend>Tipo</legend><button type="button" class:active={expense} onclick={() => setType(true)}>Gasto</button><button type="button" class:active={!expense} onclick={() => setType(false)}>Ingreso</button></fieldset>
    <label>Monto<input bind:value={amount} inputmode="decimal" placeholder="0.00" class:expense class:income={!expense}/></label>
    <label>Cuenta<select bind:value={account}><option value="">Selecciona una cuenta</option>{#each accounts as a}<option value={String(a.id)}>{a.name}</option>{/each}</select></label>
    <label>Comercio<input bind:value={payee} list="payee-list" onchange={(e) => { void prefill(e.currentTarget.value); }}/><datalist id="payee-list">{#each suggestions as item}<option value={item}></option>{/each}</datalist></label>
    <label>Categoría<select bind:value={category}><option value="">Sin categoría</option>{#each groups as group}<optgroup label={group.name}>{#each group.categories.filter(c => !c.hidden) as c}<option value={String(c.id)}>{c.name}</option>{/each}</optgroup>{/each}</select></label>
    <label>Fecha<input type="date" bind:value={date}/></label><label>Notas<input bind:value={notes}/></label>
    {#if error}<p role="alert" class="error">{error}</p>{/if}
    <div class="buttons"><button type="button" onclick={close}>Cancelar</button><button class="save" type="submit" disabled={cents === null || !account || !date || saving}>Guardar</button></div>
  </form>
</div></div>
<style>
.backdrop{position:fixed;inset:0;background:#0009;display:grid;place-items:center;z-index:10;padding:16px}.modal{background:var(--card);border:1px solid var(--border);border-radius:18px;padding:24px;width:min(520px,100%);max-height:95vh;overflow:auto}h2,p{margin:0 0 12px}p{color:var(--muted)}form{display:grid;gap:12px}label{display:grid;gap:5px;font-size:.9rem}input,select,button{font:inherit;color:inherit;background:var(--canvas);border:1px solid var(--border);border-radius:8px;padding:9px}fieldset{border:0;padding:0;margin:0}legend{font-size:.9rem}button{cursor:pointer}.active,.save{background:var(--hero);color:white}.buttons{display:flex;justify-content:flex-end;gap:8px}.expense,.error{color:var(--coral)}.income{color:var(--emerald)}button:disabled{opacity:.5;cursor:not-allowed}
</style>
