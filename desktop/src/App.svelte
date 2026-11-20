<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { onMount } from "svelte";
  import * as api from "./lib/api";
  import { app, refresh, start } from "./lib/app.svelte";
  import { runCommand } from "./lib/commands.svelte";
  import { openEntry, openImport } from "./lib/dialogs.svelte";
  import ImportDialog from "./components/ImportDialog.svelte";
  import Icon from "./components/Icon.svelte";
  import { remember, toast, undo } from "./lib/undo.svelte";
  import Palette from "./components/Palette.svelte";
  import QuickCapture from "./screens/QuickCapture.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import EntryDialog from "./components/EntryDialog.svelte";
  import Toast from "./components/Toast.svelte";
  import Resumen from "./screens/Resumen.svelte";
  import Movimientos from "./screens/Movimientos.svelte";
  import Presupuesto from "./screens/Presupuesto.svelte";
  import Proximos from "./screens/Proximos.svelte";
  import Deseos from "./screens/Deseos.svelte";
  import WishDialog from "./components/WishDialog.svelte";
  import ScheduleDialog from "./components/ScheduleDialog.svelte";
  import Ajustes from "./screens/Ajustes.svelte";

  // The quick-capture window loads this same page at #quick and shows only that.
  const quickWindow = location.hash === "#quick";
  let dropping = $state(false);
  // Dev builds only: lets desktop/qa scripts open dialogs with the app's own state.
  if (import.meta.env.DEV) Object.assign(window, { __finanzas: { openImport, runCommand } });
  onMount(() => {
    if (quickWindow) return;
    start().catch(e => app.error = String(e));
    const offs = [
      listen<string>("menu", e => runCommand(e.payload)),
      listen<{ id: number; label: string }>("quick-saved", e => { remember(e.payload.label, () => api.deleteEntry(e.payload.id).then(refresh)); void refresh(); }),
      listen<{ kind: "gasto" | "ingreso"; cents: number | null; payee: string; categoryId: number | null; accountId: number | null; date: string }>("quick-details", e => {
        const d = e.payload; openEntry(d.kind, null, { cents: d.cents, payee: d.payee, categoryId: d.categoryId, accountId: d.accountId, date: new Date(d.date) });
      }),
      // Tauri delivers dropped files with their paths; Rust reads them (only .csv, up to 5 MB).
      getCurrentWebview().onDragDropEvent(e => {
        if (e.payload.type === "enter" || e.payload.type === "over") dropping = true;
        else if (e.payload.type === "leave") dropping = false;
        else if (e.payload.type === "drop") {
          dropping = false;
          const path = e.payload.paths[0];
          if (path) api.readCsv(path).then(f => { document.querySelector<HTMLDialogElement>("dialog[open]")?.close(); openImport(f); }).catch(err => toast(String(err)));
        }
      }),
    ];
    return () => offs.forEach(off => void off.then(f => f()));
  });
  let screen = $derived(app.route.split(/[/?]/)[0]);

  // ponytail: ⌘Z only for now; the full shortcut map and native menu arrive with T7.
  function keydown(e: KeyboardEvent) {
    const typing = e.target instanceof Element && e.target.matches("input, textarea, select");
    if ((e.metaKey || e.ctrlKey) && !e.shiftKey && e.key.toLowerCase() === "z" && !typing && !document.querySelector("dialog[open]")) {
      e.preventDefault(); void undo();
    }
  }
</script>

<svelte:window onkeydown={keydown} />

{#if quickWindow}
  <QuickCapture />
{:else if app.error}
  <main class="main" style="display:grid;place-items:center;height:100vh">
    <article class="card" style="max-width:520px"><h2>No se pudo abrir la base de datos</h2><p class="muted" style="margin-top:8px">{app.error}</p></article>
  </main>
{:else}
  <div class="app">
    <Sidebar />
    <main class="main" id="main" tabindex="-1">
      <div class="page">
        {#if !app.version}
          <p class="muted">Cargando…</p>
        {:else if screen === "resumen"}
          <Resumen />
        {:else if screen === "deseos"}
          <Deseos />
        {:else if screen === "proximos"}
          <Proximos />
        {:else if screen === "presupuesto"}
          <Presupuesto />
        {:else if screen === "movimientos"}
          <Movimientos />
        {:else if screen.startsWith("cuenta-")}
          {#key screen}<Movimientos accountId={Number(screen.slice(7))} />{/key}
        {:else if screen === "ajustes"}
          <Ajustes />
        {:else}
          <Resumen />
        {/if}
      </div>
    </main>
  </div>
  <EntryDialog />
  <ScheduleDialog />
  <WishDialog />
  <Palette />
  <ImportDialog />
  {#if dropping}<div class="drop"><div><Icon name="upload" />Suelta tu CSV para importarlo<small style="font-weight:400;font-size:13px;opacity:.85">Verás una vista previa antes de guardar</small></div></div>{/if}
  <Toast />
{/if}
