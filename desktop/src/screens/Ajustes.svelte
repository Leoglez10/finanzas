<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import * as api from "../lib/api";
  import { app, go, refresh, updateSettings, profileColor, PROFILE_COLORS } from "../lib/app.svelte";
  import { SHORTCUTS } from "../lib/shortcuts";
  import { runCommand } from "../lib/commands.svelte";
  import { checkUpdates, installUpdate, restart, updates } from "../lib/updates.svelte";
  import { toast } from "../lib/undo.svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { SECURITY_URL, sendFeedback, type FeedbackKind } from "../lib/feedback";
  import Icon from "../components/Icon.svelte";
  import Topbar from "../components/Topbar.svelte";

  const TABS: [string, string, string][] = [["perfil", "Perfil", "wallet"], ["general", "General", "gear"], ["avisos", "Avisos", "bell"], ["deudas", "Deudas", "card"], ["datos", "Datos", "upload"], ["agentes", "Agentes IA", "sparkle"], ["actualizaciones", "Actualizaciones", "sparkle"], ["comentarios", "Fallas y mejoras", "alert"], ["atajos", "Atajos", "command"]];
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
  let dbPath = $state("");
  let agentList = $state<api.AgentView[]>([]);
  let copied = $state("");
  $effect(() => {
    void app.version;
    if (tab[0] === "datos") { api.importAvailability().then(v => importable = v).catch(() => importable = null); api.databasePath().then(p => dbPath = p).catch(() => dbPath = ""); }
    if (tab[0] === "agentes") api.agents().then(a => agentList = a).catch(e => toast(String(e)));
  });
  async function copy(agent: api.AgentView) {
    try { await navigator.clipboard.writeText(agent.config); copied = agent.name; setTimeout(() => copied = "", 2000); } catch (e) { toast(`No se pudo copiar: ${e}`); }
  }

  let fbKind = $state<FeedbackKind>("falla"), fbTitle = $state(""), fbBody = $state(""), fbError = $state(""), fbBusy = $state(false);
  let fbSent = $state<{ number: number; url: string } | null>(null);
  const open = (url: string) => openUrl(url).catch(e => toast(String(e)));
  async function submitFeedback(e: SubmitEvent) {
    e.preventDefault();
    if (fbBusy) return;
    fbBusy = true; fbError = "";
    try { fbSent = await sendFeedback({ kind: fbKind, title: fbTitle, body: fbBody, version }); fbTitle = ""; fbBody = ""; }
    catch (err) { fbError = err instanceof Error ? err.message : String(err); }
    finally { fbBusy = false; }
  }

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

{#snippet toggle(key: "enabled" | "tray" | "autostart" | "debts_page", title: string, text: string)}
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
    {:else if tab[0] === "deudas"}
      <article class="card">
        <div class="card-h"><h2>Deudas</h2></div>
        {@render toggle("debts_page", "Mostrar la página Deudas", "Junta tus tarjetas de crédito, préstamos y compras a meses, con cuánto pagar para no generar intereses.")}
        {#if s.debts_page}<p class="muted" style="font-size:13px">Ya aparece en la barra lateral. <button class="link" onclick={() => go("deudas")}>Abrir Deudas</button></p>{/if}
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
        <p class="muted" style="margin-bottom:10px">Movimientos de tu banco: arrastra un CSV, Excel o PDF a cualquier parte de la ventana, o elígelo aquí. Verás una vista previa y los duplicados se detectan solos.</p>
        <div style="margin-bottom:18px"><button class="btn" onclick={() => runCommand("import")}><Icon name="upload" />Elegir archivo…</button></div>
        <p class="muted" style="margin-bottom:14px">Otra app de finanzas: copia todo a un perfil vacío. El archivo de origen solo se lee, nunca se modifica.</p>
        <div style="display:flex;gap:8px;flex-wrap:wrap">
          <button class="btn" disabled={importable !== null && !importable[0]} onclick={() => runImport("actual")}><Icon name="upload" />Desde Actual Budget…</button>
          {#if importable?.[1]}<button class="btn" disabled={!importable[0]} onclick={() => runImport("legacy")}>Desde la app anterior de Mac</button>{/if}
        </div>
        {#if importable && !importable[0]}<p class="muted" style="font-size:13px;margin-top:10px">Este perfil ya tiene datos. Crea un perfil nuevo para importar.</p>{/if}
      </article>
      <article class="card">
        <div class="card-h"><h2>Base de datos</h2></div>
        <p class="muted" style="font-size:13px">Tus datos viven aquí, fuera de la app: reinstalar o actualizar no los toca. Para un respaldo, copia este archivo con la app cerrada.</p>
        <pre style="margin:10px 0 0;padding:12px;border-radius:10px;background:var(--surface-2);border:1px solid var(--border);white-space:pre-wrap;word-break:break-all;font-size:12.5px">{dbPath}</pre>
      </article>
    {:else if tab[0] === "agentes"}
      <article class="card">
        <div class="card-h"><h2>Agentes MCP</h2><button class="link" onclick={() => api.agents().then(a => agentList = a)}>Actualizar</button></div>
        <p class="muted" style="font-size:13px;margin-bottom:6px">Conecta un agente de IA para consultar y registrar tus finanzas. El punto se pone verde mientras el agente tiene Finanzas abierto.</p>
        {#each agentList as a (a.name)}
          <div class="setting" style="display:grid;gap:8px">
            <div style="display:flex;align-items:center;gap:10px"><span style="width:10px;height:10px;border-radius:50%;background:{a.connected ? 'var(--income)' : 'var(--border)'}"></span><b>{a.name}</b><span class="muted" style="font-size:13px">{a.connected ? "Conectado" : "Sin conexión"}</span></div>
            {#if !a.connected}
              <p>{a.instructions}</p>
              <pre style="margin:0;padding:12px;border-radius:10px;background:var(--surface-2);border:1px solid var(--border);white-space:pre-wrap;word-break:break-all;font-size:12.5px">{a.config}</pre>
              <div><button class="btn sm" onclick={() => copy(a)}>{copied === a.name ? "Copiado" : "Copiar configuración"}</button></div>
            {/if}
          </div>
        {/each}
      </article>
    {:else if tab[0] === "actualizaciones"}
      <article class="card">
        <div class="card-h"><h2>Actualizaciones</h2></div>
        <div class="setting" style="border:0;padding-top:0"><div class="grow"><b>Versión {version}</b>
          <p>{#if updates.status === "checking"}Buscando…{:else if updates.status === "none"}Estás al día.{:else if updates.status === "available"}Hay una versión nueva: {updates.version}.{:else if updates.status === "downloading"}Descargando {updates.version}… {Math.round(updates.progress * 100)}%{:else if updates.status === "ready"}Lista: reinicia para usar {updates.version}.{:else}La app se actualiza sola; Homebrew solo la instala.{/if}</p></div>
          {#if updates.status === "available"}<button class="btn sm primary" onclick={installUpdate}>Instalar</button>
          {:else if updates.status === "ready"}<button class="btn sm primary" onclick={restart}>Reiniciar</button>
          {:else}<button class="btn sm" disabled={updates.status === "checking" || updates.status === "downloading"} onclick={() => checkUpdates()}>Buscar actualizaciones</button>{/if}
        </div>
        {#if updates.status === "error"}<p class="err" role="alert">{updates.error}</p>{/if}
        {#if updates.notes && updates.status !== "none"}<p class="muted" style="font-size:13px;white-space:pre-line">{updates.notes}</p>{/if}
        <label class="setting"><div class="grow"><b>Buscar actualizaciones al abrir</b><p>Se descargan solo cuando tú lo pides.</p></div><input type="checkbox" role="switch" class="switch" checked={s.auto_update} onchange={e => save({ auto_update: e.currentTarget.checked })} /></label>
      </article>
    {:else if tab[0] === "comentarios"}
      <article class="card">
        <div class="card-h"><h2>Reporta una falla o propón una mejora</h2></div>
        <p class="muted" style="margin-top:0">Se publica como issue en GitHub, a la vista de todos. Solo se envía lo que escribes, la versión de la app y tu sistema operativo; nunca tus movimientos ni cuentas. Si es un problema de seguridad, no lo publiques: <button class="link" type="button" onclick={() => open(SECURITY_URL)}>escríbenos en privado</button>.</p>
        {#if fbSent}
          <div class="setting" style="border:0"><div class="grow"><b>¡Gracias! Se creó el reporte #{fbSent.number}.</b><p>Puedes seguirlo en GitHub.</p></div>
            <button class="btn sm" onclick={() => open(fbSent!.url)}>Ver en GitHub</button>
            <button class="btn sm ghost" onclick={() => fbSent = null}>Enviar otro</button></div>
        {:else}
          <form style="display:grid;gap:12px" onsubmit={submitFeedback}>
            <div class="seg" role="group" aria-label="Tipo de reporte" style="justify-self:start">
              <button type="button" aria-pressed={fbKind === "falla"} onclick={() => fbKind = "falla"}>Falla</button>
              <button type="button" aria-pressed={fbKind === "mejora"} onclick={() => fbKind = "mejora"}>Mejora</button>
            </div>
            <input class="input" aria-label="Título" placeholder={fbKind === "falla" ? "Qué falla, en pocas palabras" : "Qué te gustaría, en pocas palabras"} maxlength="120" minlength="4" required bind:value={fbTitle} />
            <textarea class="input" aria-label="Descripción" rows="6" maxlength="5000" minlength="10" required placeholder={fbKind === "falla" ? "Qué hiciste, qué esperabas y qué pasó." : "Cómo te ayudaría y en qué pantalla."} bind:value={fbBody}></textarea>
            {#if fbError}<p class="err" role="alert">{fbError}</p>{/if}
            <div><button class="btn primary" disabled={fbBusy}>{fbBusy ? "Enviando…" : "Enviar reporte"}</button></div>
          </form>
        {/if}
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
