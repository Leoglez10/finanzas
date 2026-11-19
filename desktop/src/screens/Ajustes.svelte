<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import * as api from "../lib/api";
  import { app, go, refresh, updateSettings, profileColor, PROFILE_COLORS } from "../lib/app.svelte";
  import { SHORTCUTS } from "../lib/shortcuts";
  import { toast } from "../lib/undo.svelte";
  import Icon from "../components/Icon.svelte";
  import Topbar from "../components/Topbar.svelte";

  const TABS: [string, string, string][] = [["perfil", "Perfil", "wallet"], ["general", "General", "gear"], ["avisos", "Avisos", "bell"], ["datos", "Datos", "upload"], ["actualizaciones", "Actualizaciones", "sparkle"], ["atajos", "Atajos", "command"]];
  const PALETTES: [string, string, string][] = [["petroleo", "Petróleo", "El original"], ["bosque", "Bosque", "Verde oliva sobrio"], ["ciruela", "Ciruela", "Violeta profundo"], ["grafito", "Grafito", "Casi sin color"], ["arena", "Arena", "Cálido, tonos café"], ["sistema", "Sistema", "Acento de macOS"]];
  let tab = $derived(TABS.find(t => t[0] === app.route.split("/")[1]) ?? TABS[0]);
  let s = $derived(app.settings);
  let index = $derived(Math.max(0, app.profiles.list.findIndex(p => p.slug === app.profiles.active)));
  let active = $derived(app.profiles.list[index]);
  let newName = $state(""), profileError = $state("");
  let deleting = $state<api.ProfileView | null>(null);
  let confirmDialog = $state<HTMLDialogElement>();
  let version = $state("");
  let importable = $state<[boolean, boolean] | null>(null);
  getVersion().then(v => version = v);
  $effect(() => { void app.version; if (tab[0] === "datos") api.importAvailability().then(v => importable = v).catch(() => importable = null); });

  async function save(patch: Partial<api.Settings>) { try { await updateSettings(patch); } catch (e) { toast(String(e)); } }
  const mode = $derived(s?.dark === null ? "system" : s?.dark ? "dark" : "light");
  async function profileAction(run: () => Promise<unknown>, done: string) {
    profileError = "";
    try { await run(); await refresh(); toast(done); } catch (e) { profileError = String(e); }
  }
  const switchTo = (p: api.ProfileView) => profileAction(() => api.switchProfile(p.slug), `Ahora estás en ${p.label}`);
  const create = () => profileAction(async () => { await api.createProfile(newName); newName = ""; }, "Perfil creado");
  const rename = (p: api.ProfileView, name: string) => name.trim() && name.trim() !== p.label ? profileAction(() => api.renameProfile(p.slug, name), "Perfil renombrado") : undefined;
  const toggleHidden = (p: api.ProfileView) => save({ hidden_profiles: p.hidden ? s!.hidden_profiles.filter(x => x !== p.slug) : [...s!.hidden_profiles, p.slug] });
  const setColor = (color: string) => save({ profile_colors: { ...s!.profile_colors, [app.profiles.active]: color } });
  function askDelete(p: api.ProfileView) { deleting = p; confirmDialog?.showModal(); }
  function confirmDelete(e: Event) {
    const value = (confirmDialog?.returnValue ?? "");
    if (value === "delete" && deleting) { const p = deleting; void profileAction(() => api.deleteProfile(p.slug), `Eliminaste ${p.label}`); }
    deleting = null; if (confirmDialog) confirmDialog.returnValue = "";
    void e;
  }
  async function runImport(source: "actual" | "legacy") {
    try {
      const summary = await api.importData(source);
      if (summary) { toast(`Importado: ${summary.accounts} cuentas, ${summary.entries} movimientos, ${summary.schedules} pagos programados`); await refresh(); }
    } catch (e) { toast(String(e)); }
  }
</script>

{#snippet toggle(key: "enabled" | "tray" | "autostart", title: string, text: string)}
  <label class="setting"><div class="grow"><b>{title}</b><p>{text}</p></div><input type="checkbox" role="switch" class="switch" checked={s?.[key]} onchange={e => save({ [key]: e.currentTarget.checked })} /></label>
{/snippet}
{#snippet stepper(key: "days" | "hour", label: string, value: string, min: number, max: number)}
  <div class="setting"><div class="grow"><b>{label}</b></div>
    <span class="stepper"><button aria-label="Menos" disabled={!s?.enabled || s[key] <= min} onclick={() => save({ [key]: s![key] - 1 })}>−</button><output>{value}</output><button aria-label="Más" disabled={!s?.enabled || s[key] >= max} onclick={() => save({ [key]: s![key] + 1 })}>+</button></span></div>
{/snippet}

<Topbar title="Ajustes" sub={tab[1]} />
{#if s}
<div class="settings">
  <nav class="subnav" aria-label="Secciones de ajustes">
    {#each TABS as [id, label, icon] (id)}
      <button class="nav" aria-current={id === tab[0] ? "page" : undefined} onclick={() => go(`ajustes/${id}`)}>
        {#if id === "perfil" && active}<span class="avatar" style="--c:{profileColor(active, index)};width:20px;height:20px;font-size:10px">{active.label[0]}</span>{:else}<Icon name={icon} />{/if}<span>{label}</span>
      </button>
    {/each}
  </nav>
  <div class="grid">
    {#if tab[0] === "perfil" && active}
      <article class="card">
        <div class="card-h"><h2>Tu perfil</h2></div>
        <div class="setting" style="border:0;padding-top:0;align-items:flex-start">
          <span class="avatar lg" style="--c:{profileColor(active, index)}">{active.label[0]}</span>
          <div class="grow" style="display:grid;gap:10px">
            {#key active.slug}
              <label class="field">Nombre<input class="input" value={active.label} disabled={active.slug === "personal"} onchange={e => rename(active, e.currentTarget.value)} />
                {#if active.slug === "personal"}<small>El perfil Personal es tu base principal y no se renombra.</small>{/if}</label>
            {/key}
            <div class="field">Color<div class="swatches" role="radiogroup" aria-label="Color del perfil">
              {#each PROFILE_COLORS as c, i (c)}<label style="--c:{c}"><input type="radio" name="pcolor" checked={profileColor(active, index) === c} aria-label="Color {i + 1}" onchange={() => setColor(c)} /></label>{/each}
            </div></div>
          </div>
        </div>
      </article>
      <article class="card">
        <div class="card-h"><h2>Tus perfiles</h2></div>
        <p class="muted" style="margin:-6px 0 8px;font-size:13px">Cada perfil tiene sus propias cuentas, presupuesto y deseos.</p>
        <div class="list">
          {#each app.profiles.list as p, i (p.slug)}
            <div class="row">
              <span class="avatar" style="--c:{profileColor(p, i)}">{p.label[0]}</span>
              <div class="grow"><p class="title">{p.label}</p>{#if p.hidden}<p class="meta">Oculto en el menú</p>{/if}</div>
              {#if p.slug === app.profiles.active}<span class="chip ok">En uso</span>{:else}<button class="btn sm" onclick={() => switchTo(p)}>Cambiar</button>{/if}
              {#if p.slug !== "personal"}
                <button class="btn sm ghost" onclick={() => toggleHidden(p)}>{p.hidden ? "Mostrar" : "Ocultar"}</button>
                <button class="btn icon sm ghost" aria-label="Eliminar perfil {p.label}" onclick={() => askDelete(p)}><Icon name="trash" /></button>
              {/if}
            </div>
          {/each}
        </div>
        <form class="reserve" style="margin-top:12px" onsubmit={e => { e.preventDefault(); void create(); }}>
          <input class="input" bind:value={newName} placeholder="Nombre del nuevo perfil, por ejemplo Negocio" aria-label="Nombre del nuevo perfil" />
          <button class="btn" disabled={!newName.trim()}><Icon name="plus" />Nuevo perfil</button>
        </form>
        {#if profileError}<p class="err" role="alert" style="margin-top:8px">{profileError}</p>{/if}
      </article>
    {:else if tab[0] === "general"}
      <article class="card">
        <div class="card-h"><h2>Apariencia</h2></div>
        <div class="setting" style="border:0;padding-top:0"><div class="grow"><b>Modo</b><p>Por defecto sigue al sistema.</p></div>
          <div class="seg" role="group" aria-label="Modo claro u oscuro">
            {#each [["system", "Sistema", null], ["light", "Claro", false], ["dark", "Oscuro", true]] as [id, label, dark] (id)}
              <button aria-pressed={mode === id} onclick={() => save({ dark: dark as boolean | null })}>{label}</button>
            {/each}
          </div></div>
        <div class="setting" style="display:block"><b>Tema de color</b><p>Los colores de ingresos, gastos y categorías no cambian: siempre significan lo mismo.</p>
          <div class="themes" role="radiogroup" aria-label="Tema de color">
            {#each PALETTES as [id, label, note] (id)}
              <label class="theme-card" data-palette={id}><input type="radio" name="palette" checked={s.palette === id} onchange={() => save({ palette: id })} />
                <span class="pv" aria-hidden="true"><span class="pv-side"><i></i><i></i><i></i></span><span class="pv-main"><span class="pv-hero"></span><span class="pv-row"><i></i><i></i></span></span></span>
                <span>{label}<small>{note}</small></span></label>
            {/each}
          </div></div>
      </article>
      <article class="card">
        <div class="card-h"><h2>Sistema</h2></div>
        {@render toggle("tray", "Mostrar en la barra de menú", "Saldo y próximo pago a un clic; cerrar la ventana no cierra la app.")}
        {@render toggle("autostart", "Abrir al iniciar sesión", "Así los avisos llegan aunque no abras Finanzas.")}
      </article>
    {:else if tab[0] === "avisos"}
      <article class="card">
        <div class="card-h"><h2>Avisos de pagos</h2></div>
        {@render toggle("enabled", "Avisarme antes de cada pago", "Notificación del sistema, aunque la ventana esté cerrada.")}
        {@render stepper("days", "Días de anticipación", `${s.days} ${s.days === 1 ? "día" : "días"}`, 0, 14)}
        {@render stepper("hour", "Hora del aviso", `${String(s.hour).padStart(2, "0")}:00`, 0, 23)}
        <div class="setting"><div class="grow"><b>Probar</b><p>Así se verá el aviso.</p></div><button class="btn sm" onclick={() => api.testNotification().catch(e => toast(String(e)))}>Enviar aviso de prueba</button></div>
        <p class="muted" style="font-size:13px;padding-top:6px">Los pagos marcados como “Registrar automáticamente” se anotan solos en su fecha, al abrir la app y cada hora.</p>
      </article>
    {:else if tab[0] === "datos"}
      <article class="card">
        <div class="card-h"><h2>Importar</h2></div>
        <p class="muted" style="margin-bottom:14px">Copia tus datos a una base vacía. El archivo de origen solo se lee, nunca se modifica.</p>
        <div style="display:flex;gap:8px;flex-wrap:wrap">
          <button class="btn" disabled={importable !== null && !importable[0]} onclick={() => runImport("actual")}><Icon name="upload" />Desde Actual Budget…</button>
          {#if importable?.[1]}<button class="btn" disabled={!importable[0]} onclick={() => runImport("legacy")}>Desde la app anterior de Mac</button>{/if}
        </div>
        {#if importable && !importable[0]}<p class="muted" style="font-size:13px;margin-top:10px">Este perfil ya tiene datos. Crea un perfil nuevo para importar.</p>{/if}
      </article>
    {:else if tab[0] === "actualizaciones"}
      <article class="card">
        <div class="card-h"><h2>Actualizaciones</h2></div>
        <div class="setting" style="border:0;padding-top:0"><div class="grow"><b>Versión {version}</b><p>Por ahora las actualizaciones llegan con Homebrew: <code>brew upgrade --cask finanzas</code></p></div></div>
      </article>
    {:else}
      <article class="card">
        <div class="card-h"><h2>Atajos de teclado</h2></div>
        <div class="keys">{#each SHORTCUTS as [label, keys] (label)}<span>{label}</span><kbd>{keys}</kbd>{/each}</div>
      </article>
    {/if}
  </div>
</div>
{/if}

<dialog bind:this={confirmDialog} aria-labelledby="del-title" aria-describedby="del-text" style="width:min(440px,calc(100vw - 32px))" onclose={confirmDelete}>
  <form method="dialog">
    <div class="dlg-b" style="padding-top:22px">
      <div class="ic bad"><Icon name="trash" /></div>
      <h2 id="del-title" style="font-size:18px">¿Eliminar el perfil {deleting?.label}?</h2>
      <p id="del-text" class="muted">Se borran sus cuentas, movimientos, presupuesto y deseos. Esto no se puede deshacer.</p>
    </div>
    <!-- svelte-ignore a11y_autofocus (the safe choice gets focus in a destructive confirmation) -->
    <div class="dlg-f"><button class="btn" value="delete" style="background:var(--expense);color:#fff;border:0">Eliminar para siempre</button><button class="btn" value="cancel" autofocus>Conservar</button></div>
  </form>
</dialog>
