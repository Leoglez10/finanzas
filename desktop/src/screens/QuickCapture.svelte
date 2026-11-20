<script lang="ts">
  // The quick-capture window (label "quick"): type "uber 120 ayer", Enter saves, Esc or clicking away hides it.
  import { invoke } from "@tauri-apps/api/core";
  import { emitTo, listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import * as api from "../lib/api";
  import { categoryColor, daysBetween, money, parseDay } from "../lib/format";
  import { describe, parse, quickContext, saveDraft } from "../lib/quick";
  import type { QuickContext } from "../lib/quick-parse";
  import Icon from "../components/Icon.svelte";

  let text = $state(""), message = $state(""), error = $state("");
  let ctx = $state<QuickContext | null>(null);
  let input: HTMLInputElement;
  let draft = $derived(ctx && text.trim() ? parse(text, ctx) : null);
  let categoryName = $derived(ctx?.categories.find(c => c.id === draft?.categoryId)?.name);
  let accountName = $derived(ctx?.accounts.find(a => a.id === (draft?.accountId ?? ctx?.accounts[0]?.id))?.name);
  const when = (d: Date, today: Date) => ({ 0: "hoy", "-1": "ayer", 1: "mañana" } as Record<number, string>)[daysBetween(today, d)] ?? d.toLocaleDateString("es-MX", { weekday: "short", day: "numeric", month: "short" });

  async function reset() {
    text = ""; message = ""; error = "";
    const [today, settings] = await Promise.all([api.today(), api.getSettings()]);
    document.documentElement.dataset.palette = settings.palette;
    if (settings.dark === null) delete document.documentElement.dataset.theme; else document.documentElement.dataset.theme = settings.dark ? "dark" : "light";
    ctx = await quickContext(parseDay(today));
    input?.focus();
  }
  onMount(() => { void reset(); const off = listen("quick-open", () => void reset()); return () => { void off.then(f => f()); }; });

  const hide = () => invoke("hide_quick");
  async function save() {
    if (!draft || !ctx) return;
    if (!draft.cents) { error = "Agrega un monto, por ejemplo “120”."; return; }
    try {
      const row = await saveDraft(draft, ctx);
      // The main window keeps the undo history, so it gets the saved row.
      await emitTo("main", "quick-saved", { id: row.id, label: `Guardado: ${describe(draft)}` });
      message = `Guardado: ${describe(draft)}`; text = "";
      setTimeout(hide, 700);
    } catch (e) { error = String(e); }
  }
  async function details() {
    if (!draft) return;
    await emitTo("main", "quick-details", { kind: draft.kind, cents: draft.cents, payee: draft.payee, categoryId: draft.categoryId, accountId: draft.accountId, date: draft.date.toISOString() });
    await hide();
  }
  function keydown(e: KeyboardEvent) {
    if (e.key === "Escape") { e.preventDefault(); void hide(); }
    if (e.key === "Enter") { e.preventDefault(); if (e.metaKey) void details(); else void save(); }
  }
</script>

<main class="quick-window" data-tauri-drag-region>
  <div class="pal-input" data-tauri-drag-region>
    <span style="color:var(--brand)"><Icon name="bolt" /></span>
    <input bind:this={input} bind:value={text} onkeydown={keydown} oninput={() => { error = ""; message = ""; }} autocomplete="off" spellcheck="false"
      placeholder="uber 120 ayer" aria-label="Movimiento en texto libre" aria-describedby="quick-preview" />
  </div>
  <div class="quick-preview" id="quick-preview" aria-live="polite">
    {#if message}<span class="chip ok"><Icon name="check" />{message}</span>
    {:else if draft && ctx}
      <span class="chip {draft.kind === 'gasto' ? 'bad' : 'ok'}"><Icon name={draft.kind === "gasto" ? "down" : "up"} />{draft.kind === "gasto" ? "Gasto" : "Ingreso"}</span>
      {#if draft.cents}<span class="chip"><b class="num" style="color:var(--ink)">{money(draft.cents)}</b></span>{:else}<span class="chip warn">Falta el monto</span>{/if}
      <span class="chip">{draft.payee || "Sin comercio"}</span>
      {#if categoryName}<span class="chip" style="--c:{categoryColor(draft.categoryId)}"><span class="dot"></span>{categoryName}</span>{:else}<span class="chip">Sin categoría</span>{/if}
      <span class="chip">{accountName ?? "Sin cuenta"}</span>
      <span class="chip brand"><Icon name="cal" />{when(draft.date, ctx.today)}</span>
    {:else}<span class="muted">Escribe qué compraste, cuánto y cuándo. Yo adivino el resto.</span>{/if}
    {#if error}<span class="err" role="alert">{error}</span>{/if}
  </div>
  <div class="quick-foot"><span>Prueba: <b>super 450.50 tarjeta</b> · <b>+3,200 sueldo</b></span>
    <button class="btn sm ghost first" onclick={details}>Más detalles <kbd>⌘↩</kbd></button>
    <button class="btn sm primary" onclick={save}>Guardar <kbd style="background:transparent;color:inherit;border-color:currentColor">↩</kbd></button>
  </div>
</main>

<style>
  .quick-window { height: 100vh; display: grid; grid-template-rows: auto 1fr auto; background: var(--surface); border: 1px solid var(--border); overflow: hidden; }
  .quick-window .pal-input input { font-size: 22px; font-weight: 600; }
</style>
