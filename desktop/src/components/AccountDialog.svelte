<script lang="ts">
  import { untrack } from "svelte";
  import * as api from "../lib/api";
  import { app, go, refresh } from "../lib/app.svelte";
  import { accountDialog } from "../lib/dialogs.svelte";
  import { isoDay, money } from "../lib/format";
  import { parseCents } from "../lib/quick-parse";
  import { toast } from "../lib/undo.svelte";
  import Icon from "./Icon.svelte";

  let dialog: HTMLDialogElement;
  let name = $state(""), opening = $state(""), date = $state(""), offBudget = $state(false), error = $state(""), saving = $state(false);
  $effect(() => { if (accountDialog.open) untrack(() => { name = ""; opening = ""; offBudget = false; error = ""; date = isoDay(app.today); if (!dialog.open) dialog.showModal(); }); });

  async function submit(e: SubmitEvent) {
    if ((e.submitter as HTMLButtonElement | null)?.value !== "save") return;
    e.preventDefault();
    const cents = opening.trim() ? parseCents(opening) : 0;
    if (!name.trim()) { error = "Ponle un nombre, por ejemplo “Banco” o “Tarjeta”."; return; }
    if (cents === null) { error = "Escribe un saldo válido, por ejemplo 1,250.50 o -300 si es deuda."; return; }
    saving = true;
    try {
      const id = await api.createAccount(name.trim(), offBudget, cents || null, date);
      dialog.close(); await refresh(); go(`cuenta-${id}`);
      toast(`Cuenta creada${cents ? ` con ${money(cents)}` : ""}`);
    } catch (err) { error = String(err); } finally { saving = false; }
  }
</script>

<dialog bind:this={dialog} aria-labelledby="account-title" onclose={() => accountDialog.open = false}>
  <form method="dialog" novalidate onsubmit={submit}>
    <div class="dlg-h"><div><h2 id="account-title">Nueva cuenta</h2><p>Banco, efectivo, tarjeta de crédito…</p></div>
      <button class="btn icon ghost" value="cancel" formnovalidate aria-label="Cerrar"><Icon name="x" /></button></div>
    <div class="dlg-b">
      <!-- svelte-ignore a11y_autofocus (inside a modal <dialog> autofocus picks the first field, as intended) -->
      <label class="field">Nombre<input class="input" bind:value={name} placeholder="Cuenta principal" autofocus /></label>
      <div class="two">
        <label class="field"><span>Saldo actual <span class="opt">· opcional</span></span><input class="input num" bind:value={opening} inputmode="decimal" placeholder="0.00" aria-describedby="opening-hint" /><small id="opening-hint">Negativo si es deuda, como una tarjeta.</small></label>
        <label class="field">A la fecha<input class="input" type="date" bind:value={date} /></label>
      </div>
      <label class="setting" style="padding-bottom:0"><div class="grow"><b>Fuera del presupuesto</b><p>Para inversiones o ahorros que no usas en el día a día.</p></div><input type="checkbox" role="switch" class="switch" bind:checked={offBudget} /></label>
      {#if error}<p class="err" role="alert">{error}</p>{/if}
    </div>
    <div class="dlg-f"><button class="btn" value="cancel" formnovalidate>Cancelar</button><button class="btn primary" value="save" disabled={saving}>Crear cuenta</button></div>
  </form>
</dialog>
