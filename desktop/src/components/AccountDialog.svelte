<script lang="ts">
  import { untrack } from "svelte";
  import * as api from "../lib/api";
  import { app, go, refresh } from "../lib/app.svelte";
  import { accountDialog } from "../lib/dialogs.svelte";
  import { isoDay, money } from "../lib/format";
  import { parseCents } from "../lib/quick-parse";
  import { toast } from "../lib/undo.svelte";
  import { cardForm, cardTerms } from "../lib/debts";
  import CardFields from "./CardFields.svelte";
  import StylePicker from "./StylePicker.svelte";
  import DatePicker from "./DatePicker.svelte";
  import Icon from "./Icon.svelte";

  let dialog: HTMLDialogElement;
  let name = $state(""), opening = $state(""), date = $state(""), offBudget = $state(false), error = $state(""), saving = $state(false);
  let isCard = $state(false), card = $state(cardForm(null));
  let icon = $state<string | null>(null), color = $state<string | null>(null);
  $effect(() => { if (accountDialog.open) untrack(() => { name = ""; opening = ""; offBudget = false; isCard = false; card = cardForm(null); icon = null; color = null; error = ""; date = isoDay(app.today); if (!dialog.open) dialog.showModal(); }); });

  async function submit(e: SubmitEvent) {
    if ((e.submitter as HTMLButtonElement | null)?.value !== "save") return;
    e.preventDefault();
    const cents = opening.trim() ? parseCents(opening) : 0;
    if (!name.trim()) { error = "Ponle un nombre, por ejemplo “Banco” o “Tarjeta”."; return; }
    if (cents === null) { error = "Escribe un saldo válido, por ejemplo 1,250.50 o -300 si es deuda."; return; }
    const terms = isCard ? cardTerms(card) : null;
    if (typeof terms === "string") { error = terms; return; }
    saving = true;
    try {
      const id = await api.createAccount(name.trim(), offBudget, cents || null, date, terms);
      if (icon || color) await api.setAccountStyle(id, icon, color);
      dialog.close(); await refresh(); go(`cuenta-${id}`);
      toast(`Cuenta creada${cents ? ` con ${money(cents)}` : ""}`);
    } catch (err) { error = String(err); } finally { saving = false; }
  }
</script>

<dialog bind:this={dialog} aria-labelledby="account-title" onclose={() => accountDialog.open = false}>
  <form method="dialog" novalidate onsubmit={submit}>
    <div class="dlg-h"><div><h2 id="account-title">Nueva cuenta</h2><p>Banco, efectivo, tarjeta de crédito…</p></div>
      <button class="btn icon ghost" type="button" onclick={() => dialog.close()} aria-label="Cerrar"><Icon name="x" /></button></div>
    <div class="dlg-b">
      <!-- svelte-ignore a11y_autofocus (inside a modal <dialog> autofocus picks the first field, as intended) -->
      <label class="field">Nombre<input class="input" bind:value={name} placeholder="Cuenta principal" autofocus /></label>
      <div class="two">
        <label class="field"><span>Saldo actual <span class="opt">· opcional</span></span><input class="input num" bind:value={opening} inputmode="decimal" placeholder="0.00" aria-describedby="opening-hint" /><small id="opening-hint">Negativo si es deuda, como una tarjeta.</small></label>
        <div class="field">A la fecha<DatePicker bind:value={date} label="A la fecha" /></div>
      </div>
      <StylePicker bind:icon bind:color />
      <label class="setting"><div class="grow"><b>Tarjeta de crédito</b><p>Lleva el límite, los cortes y las compras a meses.</p></div><input type="checkbox" role="switch" class="switch" bind:checked={isCard} /></label>
      {#if isCard}<CardFields bind:form={card} />{/if}
      <label class="setting" style="padding-bottom:0"><div class="grow"><b>Fuera del presupuesto</b><p>Para inversiones o ahorros que no usas en el día a día.</p></div><input type="checkbox" role="switch" class="switch" bind:checked={offBudget} /></label>
      {#if error}<p class="err" role="alert">{error}</p>{/if}
    </div>
    <div class="dlg-f"><button class="btn" type="button" onclick={() => dialog.close()}>Cancelar</button><button class="btn primary" value="save" disabled={saving}>Crear cuenta</button></div>
  </form>
</dialog>
