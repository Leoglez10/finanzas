<script lang="ts">
  // Two steps: write what you want, then confirm how the price was read (the "11,000" case).
  import { untrack } from "svelte";
  import * as api from "../lib/api";
  import { refresh } from "../lib/app.svelte";
  import { wishDialog } from "../lib/dialogs.svelte";
  import { categoryColor, money, parseDay } from "../lib/format";
  import { parseCents } from "../lib/quick-parse";
  import { remember } from "../lib/undo.svelte";
  import { KINDS, kindOf, type Kind } from "../lib/goals";
  import Icon from "./Icon.svelte";

  let dialog: HTMLDialogElement;
  let step = $state(1);
  let name = $state(""), price = $state(""), url = $state(""), notes = $state("");
  let categoryId = $state<number | null>(null);
  let kind = $state<Kind>("compra"), targetDate = $state("");
  let groups = $state<api.CategoryGroup[]>([]);
  let errors = $state<{ name?: string; price?: string; url?: string; save?: string }>({});
  let saving = $state(false);

  $effect(() => { if (wishDialog.open) untrack(fill); });
  function fill() {
    const w = wishDialog.wish;
    step = 1; errors = {}; saving = false;
    name = w?.name ?? ""; price = w ? (w.price / 100).toFixed(2) : ""; url = w?.url ?? ""; notes = w?.notes ?? "";
    categoryId = w?.categoryId ?? null;
    kind = w?.kind ?? "compra"; targetDate = w?.targetDate ?? "";
    api.categoriesGrouped().then(g => groups = g.filter(x => !x.isIncome));
    if (!dialog.open) dialog.showModal();
  }
  let cents = $derived(parseCents(price));
  let info = $derived(kindOf(kind));
  let noun = $derived(kind === "compra" ? "deseo" : "meta");
  let priceLabel = $derived(kind === "compra" ? "Precio" : kind === "fondo" ? "Monto que quieres juntar" : "Costo estimado");
  let category = $derived(groups.flatMap(g => g.categories).find(c => c.id === categoryId));

  function next() {
    errors = {};
    if (!name.trim()) errors.name = "Ponle un nombre.";
    if (!cents || cents <= 0) errors.price = "Escribe un precio válido, por ejemplo 11,000.00";
    if (url.trim() && !/^https?:\/\//.test(url.trim())) errors.url = "El enlace debe empezar con http:// o https://";
    if (!Object.keys(errors).length) step = 2;
  }
  async function save() {
    saving = true; errors = {};
    try {
      const before = wishDialog.wish;
      const saved = await api.saveWish(before?.id ?? null, { name: name.trim(), price: cents!, url: url.trim(), notes: notes.trim(), categoryId, kind, targetDate: targetDate || null });
      if (before) remember(kind === "compra" ? "Deseo actualizado" : "Meta actualizada", () => api.restoreWish(before).then(refresh));
      else remember(`${kind === "compra" ? "Deseo guardado" : "Meta guardada"}: ${saved.name}`, () => api.archiveWish(saved.id).then(refresh));
      dialog.close();
      await refresh();
    } catch (e) { errors.save = api.errorText(e); } finally { saving = false; }
  }
  function submit(e: SubmitEvent) {
    const value = (e.submitter as HTMLButtonElement | null)?.value;
    if (value === "cancel") return;
    e.preventDefault();
    if (value === "back") step = 1; else if (step === 1) next(); else void save();
  }
</script>

<dialog bind:this={dialog} aria-labelledby="wish-title" onclose={() => wishDialog.open = false}>
  <form method="dialog" novalidate onsubmit={submit}>
    <div class="steps" aria-hidden="true"><span class="on"></span><span class:on={step === 2}></span></div>
    <div class="dlg-h"><div><h2 id="wish-title">{wishDialog.wish ? `Editar ${noun}` : kind === "compra" ? "Nuevo deseo" : "Nueva meta"}</h2><p>Paso {step} de 2 · {step === 1 ? "Qué quieres lograr" : "Revisa antes de guardar"}</p></div>
      <button class="btn icon ghost" type="button" onclick={() => dialog.close()} aria-label="Cerrar"><Icon name="x" /></button></div>
    <div class="dlg-b">
      {#if step === 1}
        <div class="field"><span>¿Qué es?</span>
          <div class="kinds" role="radiogroup" aria-label="Tipo de meta">
            {#each KINDS as k (k.id)}
              <button type="button" role="radio" aria-checked={kind === k.id} onclick={() => kind = k.id}><Icon name={k.icon} />{k.label}</button>
            {/each}
          </div>
        </div>
        <label class="field">Nombre
          <!-- svelte-ignore a11y_autofocus (inside a modal <dialog> autofocus picks the first field, as intended) -->
          <input class="input" bind:value={name} placeholder={info.namePlaceholder} autofocus aria-invalid={!!errors.name || undefined} />
          {#if errors.name}<p class="err" role="alert">{errors.name}</p>{/if}</label>
        <div class="two">
          <label class="field">{priceLabel}<input class="input num" bind:value={price} inputmode="decimal" placeholder={info.pricePlaceholder} aria-describedby="wish-price-hint" aria-invalid={!!errors.price || undefined} />
            <small id="wish-price-hint">Coma para miles, punto para centavos</small>
            {#if errors.price}<p class="err" role="alert">{errors.price}</p>{/if}</label>
          <label class="field">Categoría<select class="input" bind:value={categoryId}>
            <option value={null}>Sin categoría</option>
            {#each groups as g (g.id)}<optgroup label={g.name}>{#each g.categories as c (c.id)}<option value={c.id}>{c.name}</option>{/each}</optgroup>{/each}
          </select><small>Para apartar dinero necesita una categoría.</small></label>
        </div>
        <label class="field"><span>¿Para cuándo? <span class="opt">· opcional</span></span><input class="input" type="date" bind:value={targetDate} />
          <small>Con una fecha te decimos cuánto apartar cada mes.</small></label>
        <label class="field"><span>Enlace <span class="opt">· opcional</span></span><input class="input" type="url" bind:value={url} placeholder="https://" aria-invalid={!!errors.url || undefined} />
          {#if errors.url}<p class="err" role="alert">{errors.url}</p>{/if}</label>
        <label class="field"><span>Nota <span class="opt">· opcional</span></span><textarea class="input" bind:value={notes}></textarea></label>
      {:else}
        <div class="review">
          <div><span class="muted">{info.label}</span><b>{name.trim()}</b></div>
          {#if targetDate}<div><span class="muted">Para</span><span>{parseDay(targetDate).toLocaleDateString("es-MX", { day: "numeric", month: "long", year: "numeric" })}</span></div>{/if}
          <div><span class="muted">{priceLabel} interpretado</span><b class="num" style="font-size:20px">{money(cents ?? 0)}</b></div>
          <div><span class="muted">Categoría</span>{#if category}<span class="chip" style="--c:{categoryColor(category.id)}"><span class="dot"></span>{category.name}</span>{:else}<span class="muted">Sin categoría</span>{/if}</div>
          {#if notes.trim()}<div><span class="muted">Nota</span><span>{notes.trim()}</span></div>{/if}
        </div>
        <p class="muted" style="font-size:13px">¿El precio es correcto? Si no, regresa y corrígelo.</p>
        {#if errors.save}<p class="err" role="alert">{errors.save}</p>{/if}
      {/if}
    </div>
    <div class="dlg-f">
      {#if step === 2}<button class="btn ghost" type="button" onclick={() => step = 1} style="margin-right:auto"><Icon name="left" />Atrás</button>{/if}
      <button class="btn" type="button" onclick={() => dialog.close()}>Cancelar</button>
      <button class="btn primary" value="next" disabled={saving}>{step === 1 ? "Continuar" : `Guardar ${noun}`}</button>
    </div>
  </form>
</dialog>

<style>
  .kinds { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; }
  .kinds button { display: grid; justify-items: center; gap: 6px; padding: 12px 6px; border: 1px solid var(--border); border-radius: var(--radius-sm); background: var(--surface-2); cursor: pointer; font-size: 13px; font-weight: 550; color: var(--muted); }
  .kinds button:hover { color: inherit; }
  .kinds button[aria-checked="true"] { border-color: var(--brand); background: var(--brand-soft); color: var(--brand); }
  .kinds :global(.i) { width: 22px; height: 22px; }
</style>
