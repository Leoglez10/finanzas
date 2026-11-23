<script lang="ts">
  // Edit a card's terms, or turn an existing account into a card when opened without one.
  import { untrack } from "svelte";
  import * as api from "../lib/api";
  import { app, refresh } from "../lib/app.svelte";
  import { cardDialog, openAccount } from "../lib/dialogs.svelte";
  import { cardForm, cardTerms, type CardForm } from "../lib/debts";
  import { remember } from "../lib/undo.svelte";
  import CardFields from "./CardFields.svelte";
  import Icon from "./Icon.svelte";

  let dialog: HTMLDialogElement;
  let form = $state<CardForm>(cardForm(null));
  let accountId = $state(0), error = $state(""), saving = $state(false);
  let debt = $derived(cardDialog.debt);
  let candidates = $derived(app.accounts.filter(a => !a.closed));
  $effect(() => { if (cardDialog.open) untrack(() => {
    form = cardForm(cardDialog.debt?.terms ?? null); error = ""; saving = false;
    accountId = cardDialog.debt?.accountId ?? candidates[0]?.id ?? 0;
    if (!dialog.open) dialog.showModal();
  }); });

  async function run(action: () => Promise<unknown>, label: string) {
    const before = debt?.isCard ? debt.terms : null, id = accountId;
    saving = true;
    try { await action(); remember(label, () => api.setCardTerms(id, before).then(refresh)); dialog.close(); await refresh(); }
    catch (e) { error = String(e); } finally { saving = false; }
  }
  function submit(e: SubmitEvent) {
    if ((e.submitter as HTMLButtonElement | null)?.value !== "save") return;
    e.preventDefault();
    const terms = cardTerms(form);
    if (typeof terms === "string") { error = terms; return; }
    if (!accountId) { error = "Elige una cuenta"; return; }
    void run(() => api.setCardTerms(accountId, terms), "Datos de la tarjeta guardados");
  }
</script>

<dialog bind:this={dialog} aria-labelledby="card-title" onclose={() => cardDialog.open = false}>
  <form method="dialog" novalidate onsubmit={submit}>
    <div class="dlg-h"><div><h2 id="card-title">{debt ? debt.name : "Agregar tarjeta de crédito"}</h2><p>Datos de la tarjeta</p></div>
      <button class="btn icon ghost" type="button" onclick={() => dialog.close()} aria-label="Cerrar"><Icon name="x" /></button></div>
    <div class="dlg-b">
      {#if !debt}
        {#if candidates.length}
          <label class="field">Cuenta<select class="input" bind:value={accountId}>{#each candidates as a (a.id)}<option value={a.id}>{a.name}</option>{/each}</select></label>
        {:else}<p class="muted">Primero crea la cuenta de tu tarjeta. <button class="link" type="button" onclick={() => { dialog.close(); openAccount(); }}>Nueva cuenta</button></p>{/if}
      {/if}
      <CardFields bind:form />
      {#if error}<p class="err" role="alert">{error}</p>{/if}
    </div>
    <div class="dlg-f">
      {#if debt?.isCard}<button class="btn ghost" type="button" style="margin-right:auto" disabled={saving} onclick={() => run(() => api.setCardTerms(accountId, null), `${debt.name} ya no es tarjeta`)}>Ya no es tarjeta</button>{/if}
      <button class="btn" type="button" onclick={() => dialog.close()}>Cancelar</button>
      <button class="btn primary" value="save" disabled={saving || !accountId}>Guardar</button>
    </div>
  </form>
</dialog>
