<script lang="ts">
  // Two steps: write what you want, then confirm how the price was read (the "11,000" case).
  import { untrack } from "svelte";
  import * as api from "../lib/api";
  import { refresh } from "../lib/app.svelte";
  import { wishDialog } from "../lib/dialogs.svelte";
  import { categoryColor, money } from "../lib/format";
  import { parseCents } from "../lib/quick-parse";
  import { remember } from "../lib/undo.svelte";
  import Icon from "./Icon.svelte";

  let dialog: HTMLDialogElement;
  let step = $state(1);
  let name = $state(""), price = $state(""), url = $state(""), notes = $state("");
  let categoryId = $state<number | null>(null);
  let groups = $state<api.CategoryGroup[]>([]);
  let errors = $state<{ name?: string; price?: string; url?: string; save?: string }>({});
  let saving = $state(false);

  $effect(() => { if (wishDialog.open) untrack(fill); });
  function fill() {
    const w = wishDialog.wish;
    step = 1; errors = {}; saving = false;
    name = w?.name ?? ""; price = w ? (w.price / 100).toFixed(2) : ""; url = w?.url ?? ""; notes = w?.notes ?? "";
    categoryId = w?.categoryId ?? null;
    api.categoriesGrouped().then(g => groups = g.filter(x => !x.isIncome));
    if (!dialog.open) dialog.showModal();
  }
  let cents = $derived(parseCents(price));
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
      const saved = await api.saveWish(before?.id ?? null, { name: name.trim(), price: cents!, url: url.trim(), notes: notes.trim(), categoryId });
      if (before) remember("Deseo actualizado", () => api.restoreWish(before).then(refresh));
      else remember(`Deseo guardado: ${saved.name}`, () => api.archiveWish(saved.id).then(refresh));
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
    <div class="dlg-h"><div><h2 id="wish-title">{wishDialog.wish ? "Editar deseo" : "Nuevo deseo"}</h2><p>Paso {step} de 2 · {step === 1 ? "Qué quieres comprar" : "Revisa antes de guardar"}</p></div>
      <button class="btn icon ghost" type="button" onclick={() => dialog.close()} aria-label="Cerrar"><Icon name="x" /></button></div>
    <div class="dlg-b">
      {#if step === 1}
        <label class="field">Nombre
          <!-- svelte-ignore a11y_autofocus (inside a modal <dialog> autofocus picks the first field, as intended) -->
          <input class="input" bind:value={name} placeholder="PlayStation 5" autofocus aria-invalid={!!errors.name || undefined} />
          {#if errors.name}<p class="err" role="alert">{errors.name}</p>{/if}</label>
        <div class="two">
          <label class="field">Precio<input class="input num" bind:value={price} inputmode="decimal" placeholder="11,000.00" aria-describedby="wish-price-hint" aria-invalid={!!errors.price || undefined} />
            <small id="wish-price-hint">Coma para miles, punto para centavos</small>
            {#if errors.price}<p class="err" role="alert">{errors.price}</p>{/if}</label>
          <label class="field">Categoría<select class="input" bind:value={categoryId}>
            <option value={null}>Sin categoría</option>
            {#each groups as g (g.id)}<optgroup label={g.name}>{#each g.categories as c (c.id)}<option value={c.id}>{c.name}</option>{/each}</optgroup>{/each}
          </select><small>Para apartar dinero necesita una categoría.</small></label>
        </div>
        <label class="field"><span>Enlace <span class="opt">· opcional</span></span><input class="input" type="url" bind:value={url} placeholder="https://" aria-invalid={!!errors.url || undefined} />
          {#if errors.url}<p class="err" role="alert">{errors.url}</p>{/if}</label>
        <label class="field"><span>Nota <span class="opt">· opcional</span></span><textarea class="input" bind:value={notes}></textarea></label>
      {:else}
        <div class="review">
          <div><span class="muted">Deseo</span><b>{name.trim()}</b></div>
          <div><span class="muted">Precio interpretado</span><b class="num" style="font-size:20px">{money(cents ?? 0)}</b></div>
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
      <button class="btn primary" value="next" disabled={saving}>{step === 1 ? "Continuar" : "Guardar deseo"}</button>
    </div>
  </form>
</dialog>
