<script lang="ts">
  import Icon from "./Icon.svelte";
  import { app, go, profileColor } from "../lib/app.svelte";
  import { money } from "../lib/format";
  import { openPalette } from "../lib/commands.svelte";

  const current = (id: string) => app.route === id || app.route.startsWith(id + "/") || app.route.startsWith(id + "?");
  let open = $derived(app.accounts.filter(a => !a.closed));
  let closed = $derived(app.accounts.filter(a => a.closed));
  const menu = [["resumen", "Resumen", "home"], ["movimientos", "Movimientos", "list"], ["presupuesto", "Presupuesto", "pie"], ["proximos", "Próximos pagos", "cal"], ["deseos", "Deseos", "gift"]];
  let index = $derived(Math.max(0, app.profiles.list.findIndex(p => p.slug === app.profiles.active)));
  let active = $derived(app.profiles.list[index]);
  let profile = $derived({ name: active?.label ?? "Personal", color: active ? profileColor(active, index) : "var(--violet)" });
</script>

<nav class="sidebar" aria-label="Navegación principal">
  <div class="brand" data-tauri-drag-region><div class="logo"><Icon name="chart" /></div><div><b>Finanzas</b><span>Tu dinero, en orden</span></div></div>
  <button class="cmdk" type="button" onclick={() => openPalette()}><Icon name="search" />Buscar o ejecutar…<kbd>⌘K</kbd></button>
  {#each menu as [id, label, icon] (id)}
    <button class="nav" aria-current={current(id) ? "page" : undefined} onclick={() => go(id)}>
      <Icon name={icon} /><span>{label}</span>
      {#if id === "proximos" && app.overdue}<span class="count" aria-label="{app.overdue} vencidos">{app.overdue}</span>{/if}
    </button>
  {/each}
  <div class="section">Cuentas</div>
  {#each open as a (a.id)}
    <button class="nav" aria-current={current(`cuenta-${a.id}`) ? "page" : undefined} onclick={() => go(`cuenta-${a.id}`)}>
      <Icon name={a.offBudget ? "bank" : "card"} /><span>{a.name}</span><span class="bal num" class:neg={a.balance < 0}>{money(a.balance)}</span>
    </button>
  {/each}
  {#if closed.length}
    <div class="section">Cerradas</div>
    {#each closed as a (a.id)}
      <button class="nav" aria-current={current(`cuenta-${a.id}`) ? "page" : undefined} onclick={() => go(`cuenta-${a.id}`)}><Icon name="card" /><span>{a.name}</span><span class="bal num">{money(a.balance)}</span></button>
    {/each}
  {/if}
  <div class="spacer"></div>
  <div class="side-foot">
    <button class="nav" aria-current={current("ajustes") ? "page" : undefined} onclick={() => go("ajustes")}><Icon name="gear" /><span>Ajustes</span></button>
    <button class="avatar" style="--c:{profile.color}" title="Perfil: {profile.name}" aria-label="Perfil {profile.name}, abrir ajustes del perfil" onclick={() => go("ajustes/perfil")}>{profile.name[0]}</button>
  </div>
</nav>
