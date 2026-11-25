<script lang="ts">
  // ⌘K: commands, payees, "uber 120 ayer" to register, "apartar 200 playstation", profiles and themes;
  // a payment, goal or movement by name ("gym", "editar gym") opens it to edit.
  import { untrack } from "svelte";
  import * as api from "../lib/api";
  import { app, go, refresh, updateSettings } from "../lib/app.svelte";
  import { COMMANDS, palette, runCommand } from "../lib/commands.svelte";
  import { openEntry, openSchedule, openWish } from "../lib/dialogs.svelte";
  import { dayMonth, dayNumber, money, parseDay } from "../lib/format";
  import { keyLabel } from "../lib/keys";
  import { describe, parse, quickContext, saveDraft } from "../lib/quick";
  import { norm, type QuickContext } from "../lib/quick-parse";
  import { remember, toast } from "../lib/undo.svelte";
  import Icon from "./Icon.svelte";

  interface Item { icon: string; label: string; hint: string; run: () => unknown }
  let dialog: HTMLDialogElement;
  let input: HTMLInputElement;
  let active = $state(0);
  let ctx = $state<QuickContext | null>(null);
  let wishes = $state<api.WishView[]>([]);
  let schedules = $state<api.ScheduleView[]>([]);
  let found = $state<api.EntryRow[]>([]);
  // "editar gym" and "gym" look for the same thing.
  let term = $derived(norm(palette.text.trim()).replace(/^editar\s*/, ""));
  $effect(() => {
    const t = term;
    if (t.length < 2) { found = []; return; }
    const timer = setTimeout(() => api.entries({ search: t, limit: 3 }).then(r => { if (t === term) found = r.rows; }).catch(() => found = []), 150);
    return () => clearTimeout(timer);
  });
  const THEMES: [string, string][] = [["petroleo", "Petróleo"], ["bosque", "Bosque"], ["ciruela", "Ciruela"], ["grafito", "Grafito"], ["arena", "Arena"], ["sistema", "Sistema"]];

  $effect(() => { if (palette.open) untrack(fill); });
  function fill() {
    active = 0;
    if (!dialog.open) dialog.showModal();
    input?.focus();
    Promise.all([quickContext(app.today), api.wishes(), api.schedules()]).then(([c, w, s]) => { ctx = c; wishes = w; schedules = s; });
  }

  let items = $derived.by((): Item[] => {
    const q = norm(palette.text.trim()), out: Item[] = [], edits: Item[] = [];
    if (term.length >= 2) {
      const has = (name: string) => norm(name).includes(term);
      schedules.filter(v => has(v.schedule.name)).slice(0, 3)
        .forEach(v => edits.push({ icon: "repeat", label: `Editar pago: ${v.schedule.name}`, hint: "Próximos", run: () => openSchedule(v) }));
      wishes.filter(v => has(v.wish.name)).slice(0, 3)
        .forEach(v => edits.push({ icon: "gift", label: `Editar meta: ${v.wish.name}`, hint: "Deseos", run: () => openWish(v.wish, v.items) }));
      found.forEach(r => { const d = parseDay(r.date); edits.push({ icon: "edit", label: `Editar movimiento: ${r.payeeName ?? "Sin comercio"} · ${money(r.amount, true)} · ${dayNumber(d)} ${dayMonth(d)}`, hint: "Movimientos", run: () => openEntry(r.amount < 0 ? "gasto" : "ingreso", r) }); });
    }
    // Asked for by name ("editar …"): the matches go first.
    if (/^editar\b/.test(q)) out.push(...edits);
    if (q && ctx) {
      const d = parse(palette.text, ctx);
      const wish = /apartar/.test(q) && d.cents ? wishes.find(w => norm(w.wish.name).split(" ").some(word => word.length > 2 && q.includes(word))) : undefined;
      if (wish && d.cents) {
        const total = Math.min(wish.wish.price, wish.wish.reserved + d.cents), before = wish.wish.reserved;
        out.push({ icon: "gift", label: `Apartar ${money(total - before)} para ${wish.wish.name}`, hint: "Deseos", run: async () => {
          try { await api.reserveWish(wish.wish.id, total); remember(`Apartaste ${money(total - before)} para ${wish.wish.name}`, () => api.reserveWish(wish.wish.id, before).then(refresh)); await refresh(); } catch (e) { toast(api.errorText(e)); }
        } });
      } else if (d.cents) {
        out.push({ icon: d.kind === "gasto" ? "down" : "up", label: `Registrar ${d.kind}: ${describe(d)}`, hint: "Captura", run: async () => {
          try { const row = await saveDraft(d, ctx!); remember(`Guardado: ${describe(d)}`, () => api.deleteEntry(row.id).then(refresh)); await refresh(); } catch (e) { toast(String(e)); }
        } });
      }
      [...new Set(ctx.history.map(h => h.payee))].filter(p => norm(p).includes(q)).slice(0, 3)
        .forEach(p => out.push({ icon: "list", label: `Movimientos de ${p}`, hint: "Buscar", run: () => go(`movimientos?q=${encodeURIComponent(p)}`) }));
      app.profiles.list.filter(p => p.slug !== app.profiles.active && norm(`perfil ${p.label}`).includes(q))
        .forEach(p => out.push({ icon: "repeat", label: `Cambiar al perfil ${p.label}`, hint: "Perfil", run: () => api.switchProfile(p.slug).then(refresh) }));
      THEMES.filter(([id, label]) => id !== app.settings?.palette && norm(`tema ${label}`).includes(q))
        .forEach(([id, label]) => out.push({ icon: "moon", label: `Tema: ${label}`, hint: "Apariencia", run: () => updateSettings({ palette: id }) }));
    }
    if (!/^editar\b/.test(q)) out.push(...edits);
    COMMANDS.filter(c => c.id !== "palette" && (!q || norm(c.label).includes(q)))
      .forEach(c => out.push({ icon: c.icon, label: c.label, hint: keyLabel(c.keys) || c.group, run: () => runCommand(c.id) }));
    // Labels key the list: two identical movements show once.
    return out.filter((x, i) => out.findIndex(y => y.label === x.label) === i).slice(0, 10);
  });
  $effect(() => { void items.length; active = Math.min(active, Math.max(0, items.length - 1)); });

  function choose(i: number) {
    const item = items[i]; if (!item) return;
    dialog.close();
    void item.run();
  }
  function keydown(e: KeyboardEvent) {
    if ((e.key === "ArrowDown" || e.key === "ArrowUp") && items.length) { e.preventDefault(); active = (active + (e.key === "ArrowDown" ? 1 : -1) + items.length) % items.length; }
    if (e.key === "Enter") { e.preventDefault(); choose(active); }
  }
</script>

<dialog bind:this={dialog} class="palette" aria-label="Paleta de comandos" onclose={() => { palette.open = false; palette.text = ""; }}>
  <div class="pal-input"><span class="muted"><Icon name="search" /></span>
    <input bind:this={input} bind:value={palette.text} onkeydown={keydown} oninput={() => active = 0} role="combobox" aria-expanded="true" aria-controls="pal-list"
      aria-autocomplete="list" aria-activedescendant={items.length ? `pal-${active}` : undefined} autocomplete="off" spellcheck="false" placeholder="Comando, comercio o “uber 120 ayer”" aria-label="Comando" />
    <kbd>esc</kbd></div>
  <ul id="pal-list" class="pal-list" role="listbox" aria-label="Resultados">
    {#each items as item, i (item.label)}
      <!-- svelte-ignore a11y_click_events_have_key_events (keyboard selection happens in the combobox input) -->
      <li role="option" id="pal-{i}" aria-selected={i === active} onclick={() => choose(i)} onmousemove={() => active = i}><Icon name={item.icon} /><span>{item.label}</span><span class="g">{item.hint}</span></li>
    {:else}
      <li class="empty" role="option" aria-disabled="true" aria-selected="false">Sin resultados. Prueba “netflix” o “uber 120 ayer”.</li>
    {/each}
  </ul>
</dialog>
