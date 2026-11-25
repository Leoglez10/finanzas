<script lang="ts">
  import Icon from "./Icon.svelte";
  import AccountBadge from "./AccountBadge.svelte";
  import { app, go, profileColor } from "../lib/app.svelte";
  import { money } from "../lib/format";
  import { keyLabel } from "../lib/keys";
  import { openPalette, sidebar, toggleSidebar } from "../lib/commands.svelte";
  import { openAccount, openAccountStyle } from "../lib/dialogs.svelte";
  import { showMenu } from "../lib/contextmenu";
  import { installUpdate, restart, updates } from "../lib/updates.svelte";

  const current = (id: string) => app.route === id || app.route.startsWith(id + "/") || app.route.startsWith(id + "?");
  let open = $derived(app.accounts.filter(a => !a.closed));
  let closed = $derived(app.accounts.filter(a => a.closed));
  const MENU = [["resumen", "Resumen", "home"], ["movimientos", "Movimientos", "list"], ["presupuesto", "Presupuesto", "pie"], ["proximos", "Próximos pagos", "cal"], ["deudas", "Deudas", "card"], ["deseos", "Deseos", "gift"]];
  let menu = $derived(MENU.filter(([id]) => id !== "deudas" || app.settings?.debts_page));
  let index = $derived(Math.max(0, app.profiles.list.findIndex(p => p.slug === app.profiles.active)));
  let active = $derived(app.profiles.list[index]);
  const narrowQuery = matchMedia("(width < 760px)");
  let narrow = $state(narrowQuery.matches);
  $effect(() => {
    const update = () => narrow = narrowQuery.matches;
    narrowQuery.addEventListener("change", update);
    return () => narrowQuery.removeEventListener("change", update);
  });
  let collapsed = $derived(sidebar.collapsed || narrow);
  // Collapsed rows keep their text for screen readers and show it as a tooltip.
  const tip = (text: string) => collapsed ? text : undefined;
  let profile = $derived({ name: active?.label ?? "Personal", color: active ? profileColor(active, index) : "var(--violet)" });
</script>

<nav class="sidebar" class:collapsed aria-label="Navegación principal">
  <div class="brand" data-tauri-drag-region><div class="logo"><Icon name="chart" /></div><div class="brand-text"><b>Finanzas</b><span>Tu dinero, en orden</span></div>
    {#if !narrow}<button class="side-toggle" type="button" onclick={toggleSidebar} aria-expanded={!collapsed}
      title="{collapsed ? 'Mostrar' : 'Ocultar'} la barra lateral ({keyLabel('⌘S')})" aria-label="{collapsed ? 'Mostrar' : 'Ocultar'} la barra lateral"><Icon name="sidebar" /></button>{/if}
  </div>
  <button class="cmdk" type="button" title={tip("Buscar o ejecutar")} onclick={() => openPalette()}><Icon name="search" /><span>Buscar o ejecutar…</span><kbd>{keyLabel("⌘K")}</kbd></button>
  {#each menu as [id, label, icon] (id)}
    <button class="nav" title={tip(label)} aria-current={current(id) ? "page" : undefined} onclick={() => go(id)}>
      <Icon name={icon} /><span>{label}</span>
      {#if id === "proximos" && app.overdue}<span class="count" aria-label="{app.overdue} vencidos">{app.overdue}</span>{/if}
    </button>
  {/each}
  <div class="section">Cuentas</div>
  {#each open as a (a.id)}
    <button class="nav" title={tip(a.name)} aria-current={current(`cuenta-${a.id}`) ? "page" : undefined} onclick={() => go(`cuenta-${a.id}`)}
      oncontextmenu={e => showMenu(e, [["Abrir", () => go(`cuenta-${a.id}`)], ["Apariencia…", () => openAccountStyle(a.id)]])}>
      <AccountBadge account={a} size={22} /><span>{a.name}</span><span class="bal num" class:neg={a.balance < 0}>{money(a.balance)}</span>
    </button>
  {/each}
  <button class="nav add" type="button" title={tip("Nueva cuenta")} onclick={openAccount}><Icon name="plus" /><span>Nueva cuenta</span></button>
  {#if closed.length}
    <div class="section">Cerradas</div>
    {#each closed as a (a.id)}
      <button class="nav" title={tip(a.name)} aria-current={current(`cuenta-${a.id}`) ? "page" : undefined} onclick={() => go(`cuenta-${a.id}`)}><AccountBadge account={a} size={22} /><span>{a.name}</span><span class="bal num">{money(a.balance)}</span></button>
    {/each}
  {/if}
  <div class="spacer"></div>
  {#if collapsed && (updates.status === "available" || updates.status === "ready")}
    <button class="nav upd" type="button" title={updates.status === "ready" ? "Reiniciar Finanzas para usar la actualización" : `Instalar Finanzas ${updates.version}`}
      onclick={updates.status === "ready" ? restart : installUpdate}><Icon name="sparkle" /><span>{updates.status === "ready" ? "Reiniciar Finanzas" : "Instalar actualización"}</span></button>
  {:else if !collapsed && (updates.status === "available" || updates.status === "downloading" || updates.status === "ready")}
    <div class="update" role="status">
      {#if updates.status === "available"}
        <b><Icon name="sparkle" />Finanzas {updates.version}</b><span class="muted">Hay una versión nueva.</span>
        <button class="btn sm primary" onclick={installUpdate}>Instalar actualización</button>
      {:else if updates.status === "downloading"}
        <b>Descargando {updates.version}…</b><div class="bar"><span style="width:{Math.round(updates.progress * 100)}%;animation:none"></span></div>
      {:else}
        <b><Icon name="check" />Lista para usarse</b><button class="btn sm primary" onclick={restart}>Reiniciar Finanzas</button>
      {/if}
    </div>
  {/if}
  <div class="side-foot">
    <button class="nav" title={tip("Ajustes")} aria-current={current("ajustes") ? "page" : undefined} onclick={() => go("ajustes")}><Icon name="gear" /><span>Ajustes</span></button>
    <button class="avatar" style="--c:{profile.color}" title="Perfil: {profile.name}" aria-label="Perfil {profile.name}, abrir ajustes del perfil" onclick={() => go("ajustes/perfil")}>{profile.name[0]}</button>
  </div>
</nav>
