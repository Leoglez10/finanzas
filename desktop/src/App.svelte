<script lang="ts">
  import { onMount } from "svelte";
  import { app, start } from "./lib/app.svelte";
  import { undo } from "./lib/undo.svelte";
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
  import Pending from "./screens/Pending.svelte";

  onMount(() => { start().catch(e => app.error = String(e)); });
  let screen = $derived(app.route.split(/[/?]/)[0]);
  const titles: Record<string, string> = { ajustes: "Ajustes" };

  // ponytail: ⌘Z only for now; the full shortcut map and native menu arrive with T7.
  function keydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && !e.shiftKey && e.key.toLowerCase() === "z" && !(e.target as HTMLElement).matches("input, textarea, select") && !document.querySelector("dialog[open]")) {
      e.preventDefault(); void undo();
    }
  }
</script>

<svelte:window onkeydown={keydown} />

{#if app.error}
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
        {:else}
          <Pending title={titles[screen] ?? "Finanzas"} />
        {/if}
      </div>
    </main>
  </div>
  <EntryDialog />
  <ScheduleDialog />
  <WishDialog />
  <Toast />
{/if}
