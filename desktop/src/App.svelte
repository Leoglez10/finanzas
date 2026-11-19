<script lang="ts">
  import { onMount } from "svelte";
  import { app, start } from "./lib/app.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import Resumen from "./screens/Resumen.svelte";
  import Pending from "./screens/Pending.svelte";

  onMount(() => { start().catch(e => app.error = String(e)); });
  let screen = $derived(app.route.split(/[/?]/)[0]);
  const titles: Record<string, string> = { movimientos: "Movimientos", presupuesto: "Presupuesto", proximos: "Próximos pagos", deseos: "Deseos", ajustes: "Ajustes" };
</script>

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
        {:else if screen.startsWith("cuenta-")}
          <Pending title={app.accounts.find(a => `cuenta-${a.id}` === screen)?.name ?? "Cuenta"} />
        {:else}
          <Pending title={titles[screen] ?? "Finanzas"} />
        {/if}
      </div>
    </main>
  </div>
{/if}
