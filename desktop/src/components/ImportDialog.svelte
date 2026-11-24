<script lang="ts">
  // CSV preview: pick the account, see new / duplicate / broken rows, import the checked ones in one go.
  import { untrack } from "svelte";
  import * as api from "../lib/api";
  import { app, refresh } from "../lib/app.svelte";
  import { csvRows, isDuplicate, mapRows, type ImportRow } from "../lib/csv";
  import { importDialog } from "../lib/dialogs.svelte";
  import { categoryColor, money, parseDay } from "../lib/format";
  import { quickContext } from "../lib/quick";
  import type { QuickContext } from "../lib/quick-parse";
  import { remember, toast } from "../lib/undo.svelte";
  import Icon from "./Icon.svelte";

  let dialog: HTMLDialogElement;
  let ctx = $state<QuickContext | null>(null);
  let rows = $state<ImportRow[]>([]);
  let picked = $state<boolean[]>([]);
  let accountId = $state(0);
  let existing = $state<api.EntryRow[]>([]);
  let saving = $state(false), error = $state("");

  $effect(() => { if (importDialog.open) untrack(fill); });
  async function fill() {
    error = ""; saving = false; rows = []; picked = [];
    if (!dialog.open) dialog.showModal();
    accountId = app.accounts.find(a => !a.closed)?.id ?? 0;
    ctx = await quickContext(app.today);
    rows = mapRows(importDialog.file?.rows ?? csvRows(importDialog.file?.text ?? ""), ctx);
  }
  // Duplicates depend on the target account, so they are recomputed when it changes.
  $effect(() => {
    const id = accountId, list = rows;
    if (!id || !list.length) return;
    api.entries({ accountId: id }).then(r => {
      existing = r.rows;
      picked = list.map(row => !row.error && !isDuplicate(row, r.rows));
    });
  });
  const duplicate = (row: ImportRow) => !row.error && isDuplicate(row, existing);
  let counts = $derived({ fresh: rows.filter(r => !r.error && !duplicate(r)).length, dups: rows.filter(duplicate).length, errors: rows.filter(r => r.error).length });
  let chosen = $derived(rows.filter((_, i) => picked[i]));
  const categoryName = (id: number | null) => ctx?.categories.find(c => c.id === id)?.name ?? "Sin categoría";

  async function submit(e: SubmitEvent) {
    if ((e.submitter as HTMLButtonElement | null)?.value !== "import") return;
    e.preventDefault();
    saving = true; error = "";
    try {
      const ids = await api.importEntries(chosen.map(r => ({ date: r.date!, amount: r.cents!, accountId, payee: r.payee, categoryId: r.categoryId, notes: "Importado" })));
      remember(`Importaste ${ids.length} movimiento${ids.length === 1 ? "" : "s"}`, async () => { for (const id of ids) await api.deleteEntry(id); await refresh(); });
      dialog.close();
      await refresh();
    } catch (err) { error = String(err); } finally { saving = false; }
  }
</script>

<dialog bind:this={dialog} class="import" aria-labelledby="import-title" onclose={() => importDialog.open = false}>
  <form method="dialog" onsubmit={submit}>
    <div class="dlg-h"><div><h2 id="import-title">Importar movimientos</h2><p>{importDialog.file?.name}</p></div>
      <button class="btn icon ghost" type="button" onclick={() => dialog.close()} aria-label="Cerrar"><Icon name="x" /></button></div>
    <div class="dlg-b">
      {#if !rows.length}
        <p class="muted">{ctx ? "No encontré filas en este archivo." : "Leyendo…"}</p>
      {:else}
        <div style="display:flex;gap:8px;flex-wrap:wrap">
          <span class="chip ok">{counts.fresh} nuevos</span>
          {#if counts.dups}<span class="chip warn">{counts.dups} ya registrado{counts.dups === 1 ? "" : "s"}</span>{/if}
          {#if counts.errors}<span class="chip bad">{counts.errors} con error</span>{/if}
        </div>
        <!-- svelte-ignore a11y_no_noninteractive_tabindex (a scrollable region must be keyboard-focusable to scroll) -->
        <div class="table-wrap" style="max-block-size:340px" tabindex="0" role="region" aria-label="Vista previa">
          <table>
            <thead><tr><th><span class="sr">Importar</span></th><th>Fecha</th><th>Comercio</th><th>Categoría</th><th class="r">Monto</th><th>Estado</th></tr></thead>
            <tbody>
              {#each rows as r, i (r.line)}
                <tr class:off={!picked[i]}>
                  <td><input type="checkbox" bind:checked={picked[i]} disabled={!!r.error} aria-label="Importar fila {r.line}: {r.payee}" /></td>
                  <td class="num">{r.date ? parseDay(r.date).toLocaleDateString("es-MX") : "—"}</td>
                  <td>{r.payee || "—"}</td>
                  <td><span class="chip" style="--c:{categoryColor(r.categoryId)}"><span class="dot"></span>{categoryName(r.categoryId)}</span></td>
                  <td class="r amt" class:pos={(r.cents ?? 0) > 0}>{r.cents ? money(r.cents, true) : "—"}</td>
                  <td>{#if r.error}<span class="chip bad">Fila {r.line}: {r.error}</span>{:else if duplicate(r)}<span class="chip warn">Duplicado</span>{:else}<span class="chip ok">Nuevo</span>{/if}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        <p class="muted" style="font-size:12.5px">Los duplicados (mismo día, comercio y monto en esa cuenta) quedan sin marcar. Las categorías se sugieren con tu historial.</p>
      {/if}
      {#if error}<p class="err" role="alert">{error}</p>{/if}
    </div>
    <div class="dlg-f">
      <label class="left">Cuenta destino
        <select class="input" bind:value={accountId} style="min-height:32px;width:auto">{#each app.accounts.filter(a => !a.closed) as a (a.id)}<option value={a.id}>{a.name}</option>{/each}</select></label>
      <button class="btn" type="button" onclick={() => dialog.close()}>Cancelar</button>
      <!-- svelte-ignore a11y_autofocus (the main action of the preview gets focus) -->
      <button class="btn primary" value="import" disabled={!chosen.length || saving} autofocus>Importar {chosen.length} movimiento{chosen.length === 1 ? "" : "s"}</button>
    </div>
  </form>
</dialog>
