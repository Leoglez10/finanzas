<script lang="ts">
  // Import preview: check what each column holds, pick the account, see new / duplicate / broken rows,
  // import the checked ones in one go. The column choices are remembered per bank (by header names).
  import { untrack } from "svelte";
  import * as api from "../lib/api";
  import { app, refresh } from "../lib/app.svelte";
  import { csvRows, detect, isDuplicate, mapRows, signature, withSaved, type Detection, type ImportRow, type Role, type SavedMapping } from "../lib/csv";
  import { importDialog } from "../lib/dialogs.svelte";
  import { categoryColor, money, parseDay } from "../lib/format";
  import { quickContext } from "../lib/quick";
  import type { QuickContext } from "../lib/quick-parse";
  import { remember, toast } from "../lib/undo.svelte";
  import Icon from "./Icon.svelte";

  let dialog: HTMLDialogElement;
  let ctx = $state<QuickContext | null>(null);
  let source = $state<string[][]>([]);
  let mapping = $state<Detection | null>(null);
  let rows = $derived(ctx && mapping ? mapRows(source, ctx, mapping) : []);
  let picked = $state<boolean[]>([]);
  let accountId = $state(0);
  let existing = $state<api.EntryRow[]>([]);
  let saving = $state(false), error = $state("");

  $effect(() => { if (importDialog.open) untrack(fill); });
  async function fill() {
    error = ""; saving = false; mapping = null; picked = [];
    if (!dialog.open) dialog.showModal();
    accountId = app.accounts.find(a => !a.closed)?.id ?? 0;
    ctx = await quickContext(app.today);
    source = importDialog.file?.rows ?? csvRows(importDialog.file?.text ?? "");
    const found = detect(source), key = signature(source, found);
    mapping = withSaved(found, key ? saved()[key] : null);
  }

  const SAVED = "import-mappings";
  const saved = (): Record<string, SavedMapping> => { try { return JSON.parse(localStorage.getItem(SAVED) ?? "{}"); } catch { return {}; } };
  function rememberLayout() {
    const key = mapping && signature(source, mapping);
    if (!mapping || !key) return;
    const { roles, dateOrder, decimal, flip } = mapping;
    localStorage.setItem(SAVED, JSON.stringify({ ...saved(), [key]: { roles, dateOrder, decimal, flip } }));
  }
  const ROLES: [Role, string][] = [["date", "Fecha"], ["payee", "Descripción"], ["amount", "Monto"], ["out", "Cargo"], ["in", "Abono"], ["type", "Tipo (cargo/abono)"], ["category", "Categoría"], ["ignore", "Ignorar"]];
  const columnName = (i: number) => (mapping && mapping.header >= 0 ? source[mapping.header][i]?.trim() : "") || `Columna ${i + 1}`;
  const sample = (i: number) => source.slice((mapping?.header ?? -1) + 1).map(r => r[i]?.trim()).find(Boolean) ?? "";
  // Only one column per role: picking a role elsewhere frees it here.
  function setRole(i: number, role: Role) {
    if (!mapping) return;
    mapping.roles = mapping.roles.map((r, j) => j === i ? role : r === role && role !== "ignore" ? "ignore" : r);
  }
  let missing = $derived(!mapping ? [] : [
    !mapping.roles.includes("date") && "la fecha",
    !mapping.roles.some(r => r === "amount" || r === "out" || r === "in") && "el monto",
    !mapping.roles.includes("payee") && "la descripción",
  ].filter(Boolean) as string[]);
  let showColumns = $derived(!!mapping && (mapping.dateAmbiguous || missing.length > 0));
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
      rememberLayout();
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
      {#if mapping && source.length}
        <details class="columns" open={showColumns}>
          <summary>Columnas del archivo{#if missing.length}<span class="chip bad">Falta {missing.join(", ")}</span>{:else if mapping.dateAmbiguous}<span class="chip warn">Revisa el orden de las fechas</span>{/if}</summary>
          <div class="column-grid">
            {#each mapping.roles as role, i (i)}
              <label><span class="col-name">{columnName(i)}</span>
                <select class="input" value={role} onchange={e => setRole(i, e.currentTarget.value as Role)}>{#each ROLES as [value, label] (value)}<option {value}>{label}</option>{/each}</select>
                <small class="muted">{sample(i) || "vacía"}</small></label>
            {/each}
          </div>
          <div class="column-opts">
            {#if mapping.roles.includes("date")}
              <label>Fechas como
                <select class="input" bind:value={mapping.dateOrder}><option value="dmy">día/mes/año</option><option value="mdy">mes/día/año</option></select></label>
            {/if}
            <label>Decimales con
              <select class="input" bind:value={mapping.decimal}><option value=".">punto (1,234.56)</option><option value=",">coma (1.234,56)</option></select></label>
            {#if mapping.roles.includes("amount") && !mapping.roles.includes("type")}
              <label class="check"><input type="checkbox" bind:checked={mapping.flip} />Los gastos vienen en positivo</label>
            {/if}
          </div>
          {#if mapping.dateAmbiguous}<p class="muted" style="font-size:12.5px;margin-top:8px">Todas las fechas sirven en los dos órdenes (por ejemplo 03/11). Confirma cuál usa tu banco.</p>{/if}
        </details>
      {/if}
      {#if !rows.length}
        <p class="muted">{!ctx ? "Leyendo…" : source.length ? "No encontré movimientos con estas columnas." : "No encontré filas en este archivo."}</p>
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
        <p class="muted" style="font-size:12.5px">Los duplicados (mismo día, comercio y monto en esa cuenta) quedan sin marcar. Las categorías se sugieren con tu historial. Las columnas que elijas se recuerdan para este banco.</p>
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
