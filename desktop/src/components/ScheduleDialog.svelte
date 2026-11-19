<script lang="ts">
  import { untrack } from "svelte";
  import * as api from "../lib/api";
  import { app, refresh } from "../lib/app.svelte";
  import { scheduleDialog } from "../lib/dialogs.svelte";
  import { isoDay } from "../lib/format";
  import { parseCents } from "../lib/quick-parse";
  import { remember } from "../lib/undo.svelte";
  import Icon from "./Icon.svelte";

  let dialog: HTMLDialogElement;
  let kind = $state<"gasto" | "ingreso">("gasto");
  let name = $state(""), amount = $state(""), payee = $state(""), start = $state("");
  let accountId = $state(0), categoryId = $state<number | null>(null);
  let frequency = $state<api.Frequency>("monthly"), interval = $state(1);
  let autoPost = $state(false), skipWeekend = $state(false);
  let groups = $state<api.CategoryGroup[]>([]);
  let error = $state(""), saving = $state(false);
  const FREQUENCIES: [api.Frequency, string, string][] = [["daily", "Diario", "días"], ["weekly", "Semanal", "semanas"], ["monthly", "Mensual", "meses"], ["yearly", "Anual", "años"]];

  $effect(() => { if (scheduleDialog.open) untrack(fill); });
  function fill() {
    const s = scheduleDialog.view?.schedule;
    error = ""; saving = false;
    kind = s && s.amount > 0 ? "ingreso" : "gasto";
    name = s?.name ?? ""; amount = s ? (Math.abs(s.amount) / 100).toFixed(2) : "";
    payee = scheduleDialog.view?.payee ?? "";
    accountId = s?.accountId ?? app.accounts.find(a => !a.closed)?.id ?? 0;
    categoryId = s?.categoryId ?? null;
    frequency = s?.recurrence?.frequency ?? "monthly"; interval = s?.recurrence?.interval ?? 1;
    start = s?.recurrence?.start ?? isoDay(app.today);
    autoPost = s?.autoPost ?? false; skipWeekend = s?.recurrence?.skipWeekend ?? false;
    api.categoriesGrouped().then(g => groups = g);
    if (!dialog.open) dialog.showModal();
  }

  async function save(e: SubmitEvent) {
    if ((e.submitter as HTMLButtonElement | null)?.value !== "save") return;
    e.preventDefault();
    const cents = parseCents(amount);
    if (!name.trim()) { error = "Ponle un nombre, por ejemplo “Renta”"; return; }
    if (!cents || cents < 0) { error = "Escribe un monto mayor a cero, por ejemplo 1,250.50"; return; }
    const draft: api.ScheduleDraft = { name: name.trim(), amount: kind === "gasto" ? -cents : cents, accountId, categoryId, payee: payee.trim(), frequency, interval: Math.max(1, Math.round(interval)), start, skipWeekend, autoPost };
    saving = true;
    try {
      const before = scheduleDialog.view?.schedule ?? null;
      const id = await api.saveSchedule(before?.id ?? null, draft);
      if (before) remember(`Pago “${draft.name}” actualizado`, () => api.restoreSchedule(before).then(refresh));
      else remember(`Programaste “${draft.name}”`, () => api.deleteSchedule(id).then(refresh));
      dialog.close();
      await refresh();
    } catch (err) { error = String(err); } finally { saving = false; }
  }
</script>

<dialog bind:this={dialog} aria-labelledby="schedule-title" onclose={() => scheduleDialog.open = false}>
  <form method="dialog" novalidate onsubmit={save}>
    <div class="dlg-h"><div><h2 id="schedule-title">{scheduleDialog.view ? "Editar pago programado" : "Programar pago"}</h2><p>Te aviso antes de cada fecha.</p></div>
      <button class="btn icon ghost" value="cancel" formnovalidate aria-label="Cerrar"><Icon name="x" /></button></div>
    <div class="dlg-b">
      <fieldset class="kind"><legend class="sr">Tipo</legend>
        <label><input type="radio" bind:group={kind} value="gasto" /><Icon name="down" />Pago</label>
        <label><input type="radio" bind:group={kind} value="ingreso" /><Icon name="up" />Ingreso</label>
      </fieldset>
      <div class="two">
        <!-- svelte-ignore a11y_autofocus (inside a modal <dialog> autofocus picks the first field, as intended) -->
        <label class="field">Nombre<input class="input" bind:value={name} placeholder="Renta" autofocus /></label>
        <label class="field">Monto<input class="input num" bind:value={amount} inputmode="decimal" placeholder="1,250.50" aria-describedby="schedule-hint" /><small id="schedule-hint">Coma para miles, punto para centavos</small></label>
        <label class="field">Cuenta<select class="input" bind:value={accountId}>{#each app.accounts.filter(a => !a.closed) as a (a.id)}<option value={a.id}>{a.name}</option>{/each}</select></label>
        <label class="field">Categoría
          <select class="input" bind:value={categoryId}>
            <option value={null}>Sin categoría</option>
            {#each groups as g (g.id)}<optgroup label={g.name}>{#each g.categories as c (c.id)}<option value={c.id}>{c.name}</option>{/each}</optgroup>{/each}
          </select></label>
        <label class="field">Comercio <input class="input" bind:value={payee} placeholder="Opcional" /></label>
        <label class="field">Primera fecha<input class="input" type="date" bind:value={start} /></label>
        <label class="field">Se repite<select class="input" bind:value={frequency}>{#each FREQUENCIES as [value, label] (value)}<option {value}>{label}</option>{/each}</select></label>
        <label class="field">Cada<span style="display:flex;gap:8px;align-items:center"><input class="input num" type="number" min="1" max="52" bind:value={interval} style="width:90px" /><span class="muted">{FREQUENCIES.find(f => f[0] === frequency)?.[2]}</span></span></label>
      </div>
      <label class="setting" style="border:0;padding:0"><div class="grow"><b>Registrar automáticamente</b><p>Se anota solo en su fecha, sin que hagas nada.</p></div><input type="checkbox" role="switch" class="switch" bind:checked={autoPost} /></label>
      <label class="setting" style="padding-bottom:0"><div class="grow"><b>Evitar fines de semana</b><p>Si cae en sábado o domingo, se mueve al lunes.</p></div><input type="checkbox" role="switch" class="switch" bind:checked={skipWeekend} /></label>
      {#if error}<p class="err" role="alert">{error}</p>{/if}
    </div>
    <div class="dlg-f"><span class="left"><kbd>esc</kbd> para cerrar</span>
      <button class="btn" value="cancel" formnovalidate>Cancelar</button>
      <button class="btn primary" value="save" disabled={saving}>{scheduleDialog.view ? "Guardar" : "Programar"}</button></div>
  </form>
</dialog>
