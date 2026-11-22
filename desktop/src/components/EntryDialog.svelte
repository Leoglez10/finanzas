<script lang="ts">
  import { untrack } from "svelte";
  import * as api from "../lib/api";
  import { app, refresh } from "../lib/app.svelte";
  import { entryDialog } from "../lib/dialogs.svelte";
  import { isoDay, money } from "../lib/format";
  import { parseCents } from "../lib/quick-parse";
  import { remember } from "../lib/undo.svelte";
  import Icon from "./Icon.svelte";

  let dialog: HTMLDialogElement;
  let kind = $state<"gasto" | "ingreso">("gasto");
  let amount = $state(""), payee = $state(""), notes = $state(""), date = $state("");
  let accountId = $state(0), categoryId = $state<number | null>(null);
  let groups = $state<api.CategoryGroup[]>([]), payees = $state<string[]>([]);
  let error = $state(""), saving = $state(false);
  let categoryTouched = false;

  // Only the open flag triggers this; untrack keeps the 2 s data poll from resetting a half-typed form.
  $effect(() => { if (entryDialog.open) untrack(fill); });
  function fill() {
    const row = entryDialog.row, draft = entryDialog.draft;
    kind = entryDialog.kind; error = ""; saving = false; categoryTouched = !!row;
    amount = row ? (Math.abs(row.amount) / 100).toFixed(2) : draft?.cents ? (draft.cents / 100).toFixed(2) : "";
    payee = row?.payeeName ?? draft?.payee ?? "";
    notes = row?.notes ?? "";
    date = row?.date ?? isoDay(draft?.date ?? app.today);
    accountId = row?.accountId ?? draft?.accountId ?? app.accounts.find(a => !a.closed)?.id ?? 0;
    categoryId = row?.categoryId ?? draft?.categoryId ?? null;
    api.categoriesGrouped().then(g => groups = g);
    if (!dialog.open) dialog.showModal();
  }

  async function suggest() {
    payees = payee.trim() ? await api.payeeSuggestions(payee.trim()) : [];
  }
  async function payeeChosen() {
    if (categoryTouched || !payee.trim()) return;
    const suggested = await api.suggestedCategory(payee.trim());
    if (suggested !== null) categoryId = suggested;
  }

  async function save(e: SubmitEvent) {
    if ((e.submitter as HTMLButtonElement | null)?.value !== "save") return;
    e.preventDefault();
    const cents = parseCents(amount);
    if (!cents || cents < 0) { error = "Escribe un monto mayor a cero, por ejemplo 1,250.50"; return; }
    if (!accountId) { error = "Primero crea una cuenta"; return; }
    const input: api.EntryInput = { date, amount: kind === "gasto" ? -cents : cents, accountId, payee: payee.trim(), categoryId, notes: notes.trim() };
    saving = true;
    try {
      const row = entryDialog.row;
      if (row) {
        const before = api.inputOf(row);
        await api.updateEntry(row.id, input);
        remember("Movimiento actualizado", () => api.updateEntry(row.id, before).then(refresh));
      } else {
        const created = await api.addEntry(input);
        remember(`Agregado: ${input.payee || "Sin comercio"} ${money(input.amount, true)}`, () => api.deleteEntry(created.id).then(refresh));
      }
      dialog.close();
      await refresh();
    } catch (err) { error = String(err); } finally { saving = false; }
  }
</script>

<dialog bind:this={dialog} aria-labelledby="tx-title" onclose={() => entryDialog.open = false}>
  <form method="dialog" novalidate onsubmit={save}>
    <div class="dlg-h"><div><h2 id="tx-title">{entryDialog.row ? "Editar movimiento" : "Nuevo movimiento"}</h2><p>Se registra en la cuenta que elijas.</p></div>
      <button class="btn icon ghost" type="button" onclick={() => dialog.close()} aria-label="Cerrar"><Icon name="x" /></button></div>
    <div class="dlg-b">
      <fieldset class="kind"><legend class="sr">Tipo</legend>
        <label><input type="radio" bind:group={kind} value="gasto" /><Icon name="down" />Gasto</label>
        <label><input type="radio" bind:group={kind} value="ingreso" /><Icon name="up" />Ingreso</label>
      </fieldset>
      <div>
        <!-- svelte-ignore a11y_autofocus (inside a modal <dialog> autofocus picks the first field, as intended) -->
        <label class="amount"><span>$</span><input bind:value={amount} inputmode="decimal" placeholder="0.00" autocomplete="off" aria-label="Monto" aria-describedby="amount-hint" aria-invalid={!!error || undefined} autofocus /></label>
        <p class="hint" id="amount-hint">Coma para miles, punto para centavos: 1,250.50</p>
        {#if error}<p class="err" role="alert" style="text-align:center">{error}</p>{/if}
      </div>
      <div class="two">
        <label class="field">Cuenta<select class="input" bind:value={accountId}>{#each app.accounts.filter(a => !a.closed) as a (a.id)}<option value={a.id}>{a.name}</option>{/each}</select></label>
        <label class="field">Fecha<input class="input" type="date" bind:value={date} required /></label>
        <label class="field">Comercio<input class="input" bind:value={payee} list="payee-list" placeholder="¿Dónde?" oninput={suggest} onchange={payeeChosen} /></label>
        <label class="field">Categoría
          <select class="input" bind:value={categoryId} onchange={() => categoryTouched = true}>
            <option value={null}>Sin categoría</option>
            {#each groups as g (g.id)}<optgroup label={g.name}>{#each g.categories.filter(c => !c.hidden || c.id === categoryId) as c (c.id)}<option value={c.id}>{c.name}</option>{/each}</optgroup>{/each}
          </select></label>
      </div>
      <datalist id="payee-list">{#each payees as p (p)}<option value={p}></option>{/each}</datalist>
      <label class="field"><span>Notas <span class="opt">· opcional</span></span><textarea class="input" bind:value={notes} placeholder="Algo para recordar"></textarea></label>
    </div>
    <div class="dlg-f"><span class="left"><kbd>esc</kbd> para cerrar</span>
      <button class="btn" type="button" onclick={() => dialog.close()}>Cancelar</button>
      <button class="btn primary" value="save" disabled={saving}>{entryDialog.row ? "Guardar" : "Agregar"} {kind}</button></div>
  </form>
</dialog>
