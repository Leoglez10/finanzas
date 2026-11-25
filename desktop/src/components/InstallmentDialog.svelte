<script lang="ts">
  // A purchase paid in monthly charges (MSI when the rate is 0).
  import { untrack } from "svelte";
  import * as api from "../lib/api";
  import { app, refresh } from "../lib/app.svelte";
  import { installmentDialog } from "../lib/dialogs.svelte";
  import { parsePercent } from "../lib/debts";
  import { isoDay, money } from "../lib/format";
  import { parseCents } from "../lib/quick-parse";
  import { remember } from "../lib/undo.svelte";
  import DatePicker from "./DatePicker.svelte";
  import Icon from "./Icon.svelte";

  let dialog: HTMLDialogElement;
  let name = $state(""), total = $state(""), months = $state("12"), start = $state(""), rate = $state(""), error = $state(""), saving = $state(false);
  let plan = $derived(installmentDialog.plan);
  $effect(() => { if (installmentDialog.open) untrack(() => {
    const p = installmentDialog.plan;
    name = p?.name ?? ""; total = p ? (p.total / 100).toFixed(2) : ""; months = String(p?.months ?? 12);
    start = p?.startDate ?? isoDay(app.today); rate = p?.annualRate ? String(p.annualRate / 100) : ""; error = ""; saving = false;
    if (!dialog.open) dialog.showModal();
  }); });
  let preview = $derived.by(() => { const c = parseCents(total), m = Number(months); return c && c > 0 && m >= 1 && !parsePercent(rate) ? money(Math.round(c / m)) : null; });

  async function submit(e: SubmitEvent) {
    if ((e.submitter as HTMLButtonElement | null)?.value !== "save") return;
    e.preventDefault();
    const cents = parseCents(total), n = Number(months), annualRate = parsePercent(rate);
    if (!name.trim()) { error = "Ponle un nombre, por ejemplo “Laptop”"; return; }
    if (!cents || cents <= 0) { error = "Escribe el monto total, por ejemplo 12,000"; return; }
    if (!Number.isInteger(n) || n < 1 || n > 120) { error = "Los meses van de 1 a 120"; return; }
    if (annualRate === undefined) { error = "Escribe la tasa como porcentaje, o déjala vacía si es sin intereses"; return; }
    saving = true;
    try {
      const before = plan;
      const saved = await api.saveInstallment(before?.id ?? null, { accountId: before?.accountId ?? installmentDialog.accountId, name: name.trim(), total: cents, months: n, startDate: start, annualRate: annualRate ?? 0 });
      remember(before ? `Compra “${saved.name}” actualizada` : `Agregaste “${saved.name}” a meses`, () => (before ? api.saveInstallment(before.id, before) : api.deleteInstallment(saved.id)).then(refresh));
      dialog.close(); await refresh();
    } catch (err) { error = String(err); } finally { saving = false; }
  }
</script>

<dialog bind:this={dialog} aria-labelledby="plan-title" onclose={() => installmentDialog.open = false}>
  <form method="dialog" novalidate onsubmit={submit}>
    <div class="dlg-h"><div><h2 id="plan-title">{plan ? "Editar compra a meses" : "Compra a meses"}</h2><p>Registra la compra completa como gasto en la tarjeta; aquí solo se divide.</p></div>
      <button class="btn icon ghost" type="button" onclick={() => dialog.close()} aria-label="Cerrar"><Icon name="x" /></button></div>
    <div class="dlg-b">
      <!-- svelte-ignore a11y_autofocus (inside a modal <dialog> autofocus picks the first field, as intended) -->
      <label class="field">Qué compraste<input class="input" bind:value={name} placeholder="Laptop" autofocus /></label>
      <div class="two">
        <label class="field">Monto total<input class="input num" bind:value={total} inputmode="decimal" placeholder="12,000" /></label>
        <label class="field">Meses<input class="input num" bind:value={months} inputmode="numeric" placeholder="12" /></label>
      </div>
      <div class="two">
        <div class="field">Fecha de compra<DatePicker bind:value={start} label="Fecha de compra" /></div>
        <label class="field"><span>Tasa anual <span class="opt">· vacía si es sin intereses</span></span><input class="input num" bind:value={rate} inputmode="decimal" placeholder="0 %" /></label>
      </div>
      {#if preview}<p class="muted" style="font-size:13px">Sin intereses: {preview} al mes.</p>{/if}
      {#if error}<p class="err" role="alert">{error}</p>{/if}
    </div>
    <div class="dlg-f"><button class="btn" type="button" onclick={() => dialog.close()}>Cancelar</button><button class="btn primary" value="save" disabled={saving}>Guardar</button></div>
  </form>
</dialog>
