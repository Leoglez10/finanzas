<script lang="ts">
  import * as api from "../lib/api";
  import { app, go } from "../lib/app.svelte";
  import { openAccount, openEntry } from "../lib/dialogs.svelte";
  import { money, monthKey, monthTitle, monthShort, shiftMonth, change, categoryColor, parseDay, daysBetween, dayNumber, dayMonth, monthDate, isoDay, monthSpan } from "../lib/format";
  import Icon from "../components/Icon.svelte";
  import Kpi from "../components/Kpi.svelte";
  import Money from "../components/Money.svelte";
  import MonthPicker from "../components/MonthPicker.svelte";
  import { WIDGETS, readLayout, placeAt, toggle, resize, type Slot } from "../lib/dashboard";
  import { flip } from "svelte/animate";
  import { kindOf, planText } from "../lib/goals";
  import Topbar from "../components/Topbar.svelte";

  let month = $state(monthKey(app.today));
  let range = $state(6);
  // The shown period runs from `fromMonth` to `month` (both YYYYMM, inclusive).
  let fromMonth = $state(monthKey(app.today));
  const monthsBetween = (a: number, b: number) => (Math.floor(b / 100) - Math.floor(a / 100)) * 12 + (b % 100) - (a % 100) + 1;
  let span = $derived(Math.max(1, monthsBetween(fromMonth, month)));
  let start = $derived(fromMonth);
  // The arrows move the whole period: three months back from a quarter, one from a month.
  const shift = (dir: number) => { month = shiftMonth(month, dir * span); fromMonth = shiftMonth(fromMonth, dir * span); };
  // Movimientos opens on the same months the summary shows.
  let periodQuery = $derived(span === 1 ? `mes=${month}` : `desde=${isoDay(monthDate(start))}&hasta=${isoDay(new Date(Math.floor(month / 100), month % 100, 0))}`);
  let title = $derived(monthSpan(start, month));
  let against = $derived(span === 1 ? undefined : `los ${span} meses anteriores`);
  let data = $state<{ overview: api.Overview; upcoming: api.UpcomingItem[]; wishes: api.WishView[]; reserved: number; debts: api.DebtsView } | null>(null);
  let error = $state("");

  // Widget layout: order and visibility, saved per profile. "Personalizar" turns on drag handles and hide buttons.
  let editing = $state(false);
  let storeKey = $derived(`finanzas.dashboard.${app.profiles.active}`);
  let layout = $state<Slot[]>(readLayout(null));
  $effect(() => { layout = readLayout(localStorage.getItem(storeKey)); });
  const save = (next: Slot[]) => { layout = next; localStorage.setItem(storeKey, JSON.stringify(next)); };
  /** Hiding sends a widget to the end, where edit mode shows it greyed out. */
  const hide = (id: string) => save(placeAt(toggle(layout, id), id, null, true));
  // Edit mode lists hidden widgets after the visible ones; otherwise they are not drawn.
  let available = $derived(layout.filter(s => s.id !== "deudas" || app.settings?.debts_page));
  let shown = $derived([...available.filter(s => !s.hidden), ...(editing ? available.filter(s => s.hidden) : [])]);
  const widgetTitle = (id: string) => WIDGETS.find(w => w.id === id)?.title ?? id;
  let dragId = $state<string | null>(null);
  // While dragging, a copy of the widget floats under the pointer; its slot in the grid stays as a dashed gap.
  // Its position is written straight to the element once per frame, outside Svelte's reactivity.
  let ghost = $state<{ id: string; x: number; y: number; w: number; h: number } | null>(null);
  let floatEl = $state<HTMLDivElement>();
  const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const widgetOf = (e: PointerEvent) => (e.currentTarget as HTMLElement).closest(".w") as HTMLElement;
  /** Follows the pointer until it is released, then saves the layout. */
  function track(onMove: (ev: PointerEvent) => void, onEnd = () => {}) {
    const end = () => { removeEventListener("pointermove", onMove); onEnd(); save(layout); };
    addEventListener("pointermove", onMove);
    addEventListener("pointerup", end, { once: true });
  }
  // Pointer-based drag (Tauri takes HTML drag-and-drop for file drops): the widget moves live to wherever the pointer is.
  function startDrag(e: PointerEvent, id: string) {
    e.preventDefault();
    const r = widgetOf(e).getBoundingClientRect();
    dragId = id;
    const dx = e.clientX - r.left, dy = e.clientY - r.top;
    ghost = { id, x: r.left, y: r.top, w: r.width, h: r.height };
    const grid = widgetOf(e).parentElement!;
    let settleUntil = 0, frame = 0;
    track(ev => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        if (floatEl) floatEl.style.translate = `${ev.clientX - dx}px ${ev.clientY - dy}px`;
        // While the others slide (animate:flip) their boxes are mid-move; wait so the drop spot does not flicker.
        if (performance.now() < settleUntil) return;
        const next = dropSpot(grid, id, ev.clientX, ev.clientY);
        if (next !== layout) { layout = next; settleUntil = performance.now() + FLIP_MS + 30; }
      });
    }, () => { cancelAnimationFrame(frame); dragId = null; ghost = null; });
  }
  const FLIP_MS = 220;
  /** The layout with `id` dropped next to the widget nearest the pointer: before it when the pointer is on its
   *  left (or above it), after it otherwise, and at the end when the pointer is below every widget. */
  function dropSpot(grid: HTMLElement, id: string, x: number, y: number): Slot[] {
    const boxes = [...grid.querySelectorAll<HTMLElement>("[data-widget]:not(.off)")].filter(el => el.dataset.widget !== id).map(el => ({ id: el.dataset.widget!, r: el.getBoundingClientRect() }));
    if (!boxes.length) return layout;
    if (y > Math.max(...boxes.map(b => b.r.bottom))) return placeAt(layout, id, null, true);
    const gapTo = (r: DOMRect) => Math.hypot(Math.max(r.left - x, 0, x - r.right), Math.max(r.top - y, 0, y - r.bottom));
    const near = boxes.reduce((a, b) => gapTo(b.r) < gapTo(a.r) ? b : a);
    const inRow = y >= near.r.top && y <= near.r.bottom;
    return placeAt(layout, id, near.id, inRow ? x > near.r.left + near.r.width / 2 : y > near.r.top + near.r.height / 2);
  }
  // The corner grip snaps the width to whole columns of the 6-column grid. While dragging only an outline of the new
  // width is drawn (written straight to the DOM); the grid reflows once, on release, instead of at every column.
  function startResize(e: PointerEvent, id: string) {
    e.preventDefault();
    const cell = widgetOf(e);
    const left = cell.getBoundingClientRect().left;
    const grid = cell.parentElement!;
    const gap = parseFloat(getComputedStyle(grid).columnGap) || 0;
    const column = (grid.clientWidth + gap) / 6;
    const start = layout.find(s => s.id === id)!.span;
    let span = start, frame = 0;
    const outline = Object.assign(document.createElement("div"), { className: "size-preview" });
    const draw = () => { outline.style.width = `${span * column - gap}px`; outline.dataset.label = `${span} de 6 columnas`; };
    draw();
    cell.append(outline);
    dragId = id;
    track(ev => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const next = Math.max(2, Math.min(6, Math.round((ev.clientX - left + gap) / column)));
        if (next !== span) { span = next; draw(); }
      });
    }, () => {
      cancelAnimationFrame(frame);
      outline.remove();
      dragId = null;
      if (span !== start) layout = resize(layout, id, span);
    });
  }

  $effect(() => {
    void app.version; const m = month, r = range, n = span;
    Promise.all([api.overview(m, r, n), api.upcoming(), api.wishes(), api.debts()])
      .then(([overview, upcoming, wishes, debts]) => { data = { overview, upcoming, wishes, reserved: wishes.reduce((s, w) => s + w.wish.reserved, 0), debts }; error = ""; })
      .catch(e => error = String(e));
  });

  let o = $derived(data?.overview);
  let income = $derived(o?.current.income ?? 0);
  let spent = $derived(o?.current.expenses ?? 0);
  // Still to pay: scheduled outflows dated in the shown months.
  let pending = $derived(data ? -data.upcoming.filter(u => { const k = monthKey(parseDay(u.date)); return u.amount < 0 && k >= start && k <= month; }).reduce((s, u) => s + u.amount, 0) : 0);
  let free = $derived(income - spent - pending);
  const pct = (n: number) => income > 0 ? Math.max(0, Math.min(100, n / income * 100)) : 0;
  let stops = $derived.by(() => {
    if (!o || !o.spendingTotal) return "var(--surface-2) 0 100%";
    let at = 0;
    return o.spending.map(c => { const from = at; at += c.spent / o.spendingTotal * 100; return `${categoryColor(c.categoryId)} ${from}% ${at}%`; }).join(",");
  });
  let maxBar = $derived(o ? Math.max(1, ...o.chart.map(p => Math.max(p.income, p.expenses))) * 1.08 : 1);
  let average = $derived(o && o.chart.length ? o.chart.reduce((s, p) => s + p.income - p.expenses, 0) / o.chart.length : 0);
</script>

<Topbar {title} sub="Resumen">
  {#snippet lead()}
    <div class="month">
      <button class="btn icon" aria-label="Periodo anterior" onclick={() => shift(-1)}><Icon name="left" /></button>
      <button class="btn icon" aria-label="Periodo siguiente" onclick={() => shift(1)}><Icon name="right" /></button>
      <MonthPicker from={fromMonth} to={month} today={app.today} onchange={(f, t) => { fromMonth = f; month = t; }} />
    </div>
  {/snippet}
  {#snippet actions()}
    <button class="btn" class:primary={editing} aria-pressed={editing} onclick={() => editing = !editing}><Icon name={editing ? "check" : "edit"} />{editing ? "Listo" : "Personalizar"}</button>
    <button class="btn" onclick={() => openEntry("ingreso")}><Icon name="plus" />Ingreso</button>
    <button class="btn primary" onclick={() => openEntry("gasto")}><Icon name="minus" />Gasto</button>
  {/snippet}
</Topbar>

{#if error}<p class="err" role="alert">{error}</p>{/if}
{#if o && data}
  {#if !o.firstRun.hasAccount || !o.firstRun.hasCategorizedIncome || !o.firstRun.hasBudget}
    <article class="card">
      <div class="card-h"><h2>Primeros pasos</h2></div>
      <div class="list">
        {#each [[o.firstRun.hasAccount, "Crea una cuenta", "Tu banco, efectivo o tarjeta.", "Nueva cuenta", openAccount], [o.firstRun.hasCategorizedIncome, "Registra un ingreso con categoría", "Así el presupuesto sabe cuánto puedes asignar.", "Nuevo ingreso", () => openEntry("ingreso")], [o.firstRun.hasBudget, "Asigna tu presupuesto", "Dale un destino a cada peso.", "Ir a Presupuesto", () => go("presupuesto")]] as [done, title, text, label, run] (title)}
          <div class="row">
            <span class="ic sm {done ? 'ok' : ''}"><Icon name={done ? "check" : "right"} /></span>
            <div class="grow"><p class="title" style:text-decoration={done ? "line-through" : "none"}>{title}</p><p class="meta">{text}</p></div>
            {#if !done}<button class="btn sm primary" onclick={run as () => void}>{label}</button>{/if}
          </div>
        {/each}
      </div>
    </article>
  {/if}
  {#if editing}
    <div class="dash-tray" role="region" aria-label="Personalizar resumen">
      <span class="muted">Arrastra con <Icon name="list" /> para mover · <Icon name="x" /> para ocultar · los ocultos quedan abajo en gris.</span>
      <button class="btn sm ghost" style="margin-left:auto" onclick={() => save(readLayout(null))}>Restablecer</button>
    </div>
  {/if}
  <div class="dash" class:editing class:moving={dragId !== null}>
    {#each shown as s (s.id)}
      <div class="w" class:off={s.hidden} style:grid-column="span {s.span}" class:dragging={ghost?.id === s.id} class:sizing={dragId === s.id && !ghost} data-widget={s.id} animate:flip={{ duration: reduceMotion ? 0 : FLIP_MS }}>
        {#if s.hidden}
          <div class="w-tools"><button class="btn sm" aria-label="Mostrar {widgetTitle(s.id)}" onclick={() => save(toggle(layout, s.id))}><Icon name="plus" />Mostrar</button></div>
        {:else if editing}
          <div class="w-tools">
            <button class="btn icon sm handle" aria-label="Mover {widgetTitle(s.id)}" onpointerdown={e => startDrag(e, s.id)}><Icon name="list" /></button>
            <button class="btn icon sm" aria-label="Hacer más angosto {widgetTitle(s.id)}" disabled={s.span <= 2} onclick={() => save(resize(layout, s.id, s.span - 1))}><Icon name="minus" /></button>
            <button class="btn icon sm" aria-label="Hacer más ancho {widgetTitle(s.id)}" disabled={s.span >= 6} onclick={() => save(resize(layout, s.id, s.span + 1))}><Icon name="plus" /></button>
            <button class="btn icon sm" aria-label="Ocultar {widgetTitle(s.id)}" onclick={() => hide(s.id)}><Icon name="x" /></button>
          </div>
          <span class="grip" aria-hidden="true" title="Arrastra para cambiar el tamaño" onpointerdown={e => startResize(e, s.id)}></span>
        {/if}
        {@render widget(s.id)}
      </div>
    {/each}
  </div>
  {#if ghost}
    <div bind:this={floatEl} class="floating" aria-hidden="true" style="width:{ghost.w}px;height:{ghost.h}px;translate:{ghost.x}px {ghost.y}px">{@render widget(ghost.id)}</div>
  {/if}
{/if}

{#snippet widget(id: string)}
  {#if o && data}
    {#if id === "hero"}
    <article class="card hero">
      <p class="label">{free >= 0 ? "Te sobra" : "Te falta"} {span === 1 ? "este mes" : "en el periodo"}</p>
      <Money cents={free} />
      <p class="note">Después de pagar lo programado · {pct(free).toFixed(0)}% de tus ingresos</p>
      <div class="flow" role="img" aria-label="Gastado {pct(spent).toFixed(0)}%, por pagar {pct(pending).toFixed(0)}%, libre {pct(free).toFixed(0)}%">
        <span style="width:{pct(spent)}%;background:#fff"></span><span style="width:{pct(pending)}%;background:rgb(255 255 255 / .55)"></span><span style="width:{pct(free)}%;background:#7CF0C0"></span>
      </div>
      <div class="legend">
        <div><p><i style="background:rgb(255 255 255 / .3)"></i>Ingresos</p><b>{money(income)}</b></div>
        <div><p><i style="background:#fff"></i>Gastado</p><b>{money(spent)}</b></div>
        <div><p><i style="background:rgb(255 255 255 / .55)"></i>Por pagar</p><b>{money(pending)}</b></div>
        <div><p><i style="background:#7CF0C0"></i>Libre</p><b>{money(free)}</b></div>
      </div>
    </article>
    {:else if id === "pagos"}
    <article class="card">
      <div class="card-h"><h2>Pagos por hacer</h2><button class="link" onclick={() => go("proximos")}>Ver todos</button></div>
      <div class="list">
        {#each data.upcoming.slice(0, 3) as u (u.scheduleId + u.date)}
          {@const d = parseDay(u.date)}{@const days = daysBetween(app.today, d)}
          <div class="row">
            <div class="date" class:over={u.overdue}><b>{dayNumber(d)}</b><small>{dayMonth(d)}</small></div>
            <div class="grow"><p class="title">{u.name}</p>
              <p class="meta">{#if u.overdue}<span class="chip bad">Vencido hace {-days} d</span>{:else if days === 0}<span class="chip warn">Hoy</span>{:else}en {days} días{/if}{#if u.autoPost}{" · automático"}{/if}</p></div>
            <b class="num" class:pos={u.amount > 0}>{money(u.amount, true)}</b>
          </div>
        {:else}<p class="muted">Nada programado en los próximos 30 días.</p>{/each}
      </div>
    </article>
    {:else if id === "kpis"}
  <section class="kpis">
    <Kpi icon="wallet" label="Saldo en cuentas" cents={o.totalBalance} />
    <Kpi icon="up" tone="ok" label="Ingresos" cents={income} note={change(o.incomeChange, shiftMonth(month, -1), against)} />
    <Kpi icon="down" tone="bad" label="Gastos" cents={spent} note={change(o.expensesChange, shiftMonth(month, -1), against)} />
    <Kpi icon="gift" tone="violet" label="Apartado en deseos" cents={data.reserved} />
  </section>
    {:else if id === "gasto"}
    <article class="card">
      <div class="card-h"><h2>¿En qué se va el dinero?</h2><button class="link" onclick={() => go(`movimientos?${periodQuery}`)}>Movimientos</button></div>
      {#if o.spending.length}
        <div style="display:flex;gap:22px;align-items:center;flex-wrap:wrap">
          <div class="donut" style="background:conic-gradient({stops})" role="img" aria-label="Gasto por categoría"><div><span><b class="num">{money(o.spendingTotal)}</b><small>gastado</small></span></div></div>
          <div class="cats">
            {#each o.spending as c (c.categoryId)}
              <button class="cat" style="--c:{categoryColor(c.categoryId)}" onclick={() => go(`movimientos?${periodQuery}&categoria=${c.categoryId}`)}>
                <i></i><span>{c.name}</span><span class="muted num">{(c.spent / o.spendingTotal * 100).toFixed(0)}%</span><b class="num">{money(c.spent)}</b>
              </button>
            {/each}
          </div>
        </div>
      {:else}<p class="muted">Sin gastos con categoría {span === 1 ? "este mes" : "en el periodo"}.</p>{/if}
    </article>
    {:else if id === "chart"}
    <article class="card">
      <div class="card-h"><h2>Ingresos vs gastos</h2>
        <div class="seg" style="margin-left:auto" role="group" aria-label="Rango">
          <button aria-pressed={range === 6} onclick={() => range = 6}>6 meses</button><button aria-pressed={range === 12} onclick={() => range = 12}>12 meses</button>
        </div>
      </div>
      <div class="chart" class:dense={o.chart.length > 6}>
        {#each o.chart as p (p.month)}
          {@const name = monthDate(p.month).toLocaleDateString("es-MX", { month: "long", year: "numeric" })}
          <button class="col" class:cur={p.month >= start && p.month <= month} onclick={() => go(`movimientos?mes=${p.month}`)} aria-label="{name}: ingresos {money(p.income)}, gastos {money(p.expenses)}. Ver movimientos">
            <span class="pair"><span style="height:{p.income / maxBar * 100}%;background:var(--income)"></span><span style="height:{p.expenses / maxBar * 100}%;background:var(--expense)"></span></span>
            <small>{monthShort(p.month)}</small>
            <span class="tip" aria-hidden="true"><b style="text-transform:capitalize">{name}</b><span>Ingresos <b>{money(p.income)}</b></span><span>Gastos <b>{money(p.expenses)}</b></span><span>Neto <b>{money(p.income - p.expenses, true)}</b></span><small style="opacity:.7">Clic para ver movimientos</small></span>
          </button>
        {/each}
      </div>
      <div style="display:flex;justify-content:space-between;margin-top:14px;padding-top:14px;border-top:1px solid var(--border)"><span class="muted">Ahorro promedio al mes</span><b class="num" class:pos={average >= 0} class:neg={average < 0}>{money(Math.round(average))}</b></div>
    </article>
    {:else if id === "presupuesto"}
      <article class="card">
        <div class="card-h"><h2>Presupuesto · {monthTitle(month)}</h2><button class="link" onclick={() => go("presupuesto")}>Ver todo</button></div>
        <div class="list">
          {#each o.budgetProgress as b (b.categoryId)}
            {@const share = b.budgeted > 0 ? Math.max(0, Math.min(100, b.spent / b.budgeted * 100)) : 0}
            <div class="row" style="display:grid;gap:6px">
              <div style="display:flex;justify-content:space-between;gap:8px"><b>{b.name}</b><span class="muted num">{money(Math.max(0, b.spent))} de {money(b.budgeted)}</span></div>
              <div class="bar" role="img" aria-label="{b.name}: {share.toFixed(0)}% usado"><span style="width:{share}%;--c:{b.spent > b.budgeted ? 'var(--expense)' : categoryColor(b.categoryId)}"></span></div>
            </div>
          {:else}<p class="muted">Aún no asignas presupuesto este mes.</p>{/each}
        </div>
      </article>
    {:else if id === "recientes"}
      <article class="card">
        <div class="card-h"><h2>Últimos movimientos</h2><button class="link" onclick={() => go("movimientos")}>Ver todos</button></div>
        <div class="list">
          {#each o.recentEntries as r (r.id)}
            <div class="row">
              <span class="dot" style="width:10px;height:10px;border-radius:50%;background:{categoryColor(r.categoryId)}"></span>
              <div class="grow"><p class="title">{r.payeeName ?? "Sin comercio"}</p><p class="meta">{r.categoryName ?? "Sin categoría"} · {dayNumber(parseDay(r.date))} {dayMonth(parseDay(r.date))}</p></div>
              <b class="num" class:pos={r.amount > 0}>{money(r.amount, true)}</b>
            </div>
          {:else}<p class="muted">Aún no hay movimientos.</p>{/each}
        </div>
      </article>
    {:else if id === "cuentas"}
      <article class="card">
        <div class="card-h"><h2>Saldo por cuenta</h2></div>
        <div class="list">
          {#each app.accounts.filter(a => !a.closed) as a (a.id)}
            <button class="row" style="width:100%;background:none;border-left:0;border-right:0;border-bottom:0;cursor:pointer;text-align:left;font:inherit;color:inherit" onclick={() => go(`cuenta-${a.id}`)}>
              <span class="ic sm"><Icon name={a.offBudget ? "chart" : "card"} /></span>
              <div class="grow"><p class="title">{a.name}</p>{#if a.offBudget}<p class="meta">Fuera del presupuesto</p>{/if}</div>
              <b class="num" class:neg={a.balance < 0}>{money(a.balance)}</b>
            </button>
          {:else}<p class="muted">Aún no tienes cuentas.</p>{/each}
        </div>
      </article>
    {:else if id === "deudas"}
      <article class="card">
        <div class="card-h"><h2>Deudas</h2><button class="link" onclick={() => go("deudas")}>Ver todas</button></div>
        {#if data.debts.debts.length}
          <p style="margin:0 0 8px"><span class="muted">Debes</span> <b class="num" style="font-size:20px">{money(data.debts.total)}</b>{#if data.debts.monthlyInstallments}<span class="muted"> · {money(data.debts.monthlyInstallments)} al mes a meses</span>{/if}</p>
          <div class="list">
            {#each data.debts.debts.slice(0, 4) as d (d.accountId)}
              {@const pct = Math.min(100, Math.round((d.utilization ?? 0) * 100))}
              <div class="row" style="display:grid;gap:6px">
                <div style="display:flex;justify-content:space-between;gap:8px"><b>{d.name}</b><span class="num">{money(d.debt)}</span></div>
                {#if d.utilization !== null}<div class="bar" role="img" aria-label="{d.name}: usas {pct}% del límite"><span style="width:{pct}%;--c:{pct > 30 ? 'var(--warn)' : 'var(--brand)'}"></span></div>{/if}
                {#if d.overdue}<p class="meta neg" style="margin:0">Pago vencido: {money(d.toAvoidInterest ?? 0)}</p>
                {:else if (d.toAvoidInterest ?? 0) > 0}<p class="meta" style="margin:0">Paga {money(d.toAvoidInterest!)}{d.dueDate ? ` antes del ${dayNumber(parseDay(d.dueDate))} ${dayMonth(parseDay(d.dueDate))}` : ""} para no pagar intereses</p>
                {:else if d.missing.length}<p class="meta" style="margin:0">Completa sus datos para recibir recomendaciones</p>{/if}
              </div>
            {/each}
          </div>
        {:else}<p class="muted">No tienes deudas.</p>{/if}
      </article>
    {:else if id === "deseos"}
      <article class="card">
        <div class="card-h"><h2>Deseos y metas</h2><button class="link" onclick={() => go("deseos")}>Ver todos</button></div>
        <div class="list">
          {#each data.wishes.slice(0, 5) as { wish, plan } (wish.id)}
            {@const share = wish.price > 0 ? Math.min(100, wish.reserved / wish.price * 100) : 0}
            <div class="row" style="display:grid;gap:6px">
              <div style="display:flex;justify-content:space-between;gap:8px"><b style="display:inline-flex;align-items:center;gap:6px"><Icon name={kindOf(wish.kind).icon} />{wish.name}</b><span class="muted num">{money(wish.reserved)} de {money(wish.price)}</span></div>
              <div class="bar" role="img" aria-label="{wish.name}: {share.toFixed(0)}% apartado"><span style="width:{share}%;--c:var(--violet)"></span></div>
              {#if plan}<p class="meta" style="margin:0" class:neg={plan.overdue}>{planText(plan)}</p>{/if}
            </div>
          {:else}<p class="muted">Sin deseos guardados.</p>{/each}
        </div>
      </article>
    {/if}
  {/if}
{/snippet}

<style>
  .dash { display: grid; grid-template-columns: repeat(6, minmax(0, 1fr)); gap: 20px; grid-auto-flow: row dense; }
  .w { position: relative; min-width: 0; contain: layout style; }
  .moving :global(*) { transition: none !important; }
  .w > :global(.card) { height: 100%; }
  @media (width < 1000px) { .w { grid-column: span 6 !important; } }
  .editing .w { outline: 2px dashed var(--border); outline-offset: 4px; border-radius: var(--radius); }
  .editing .w > :not(.w-tools, .grip) { pointer-events: none; }
  .moving { user-select: none; cursor: grabbing; }
  .w.off > :not(.w-tools) { filter: grayscale(1); opacity: .4; }
  .w.dragging { outline: 2px dashed var(--brand); background: var(--brand-soft); }
  .w.dragging > :not(.w-tools, .grip) { opacity: 0; }
  .w.sizing { outline-color: var(--brand); z-index: 5; }
  .w :global(.size-preview) { position: absolute; left: 0; top: 0; bottom: 0; z-index: 3; pointer-events: none; border: 2px dashed var(--brand); border-radius: var(--radius); background: color-mix(in srgb, var(--brand) 10%, transparent); }
  .w :global(.size-preview)::after { content: attr(data-label); position: absolute; right: 10px; bottom: 10px; padding: 3px 8px; border-radius: 99px; background: var(--brand); color: var(--brand-ink); font-size: 12px; font-weight: 600; }
  .grip { position: absolute; right: 6px; bottom: 6px; z-index: 2; width: 16px; height: 16px; border-right: 3px solid var(--brand); border-bottom: 3px solid var(--brand); border-radius: 0 0 6px 0; cursor: ew-resize; touch-action: none; }
  .floating { position: fixed; left: 0; top: 0; z-index: 50; pointer-events: none; will-change: translate; rotate: 1.5deg; scale: 1.03; border-radius: var(--radius); background: var(--canvas); box-shadow: 0 24px 60px rgb(15 30 45 / .28), 0 6px 16px rgb(15 30 45 / .12); animation: lift .18s ease-out; }
  .floating > :global(*) { height: 100%; }
  @keyframes lift { from { rotate: 0deg; scale: 1; } }
  @media (prefers-reduced-motion: reduce) { .floating { animation: none; rotate: none; scale: none; } }
  /* The tools float on the widget's top edge, so they never cover its title or links. */
  .w-tools { position: absolute; top: -17px; right: 14px; z-index: 4; display: flex; gap: 2px; padding: 2px; border-radius: 10px; background: var(--surface); border: 1px solid var(--border); box-shadow: var(--shadow); }
  .w-tools .btn { border: 0; background: none; box-shadow: none; color: var(--text, inherit); }
  .editing { row-gap: 34px; padding-top: 14px; }
  .handle { cursor: grab; touch-action: none; }
  .dash-tray { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin-bottom: 16px; padding: 12px 14px; border: 1px dashed var(--border); border-radius: var(--radius); background: var(--surface); }
  .dash-tray .muted { display: inline-flex; align-items: center; gap: 4px; font-size: 13px; }
</style>
