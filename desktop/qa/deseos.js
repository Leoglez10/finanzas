// Deseos: reserve/release with undo, Spanish core errors, buy + undo, two-step editor ("11,000"), archive + undo, trip goal with a date.
(async () => {
  const wait = ms => window.__TAURI_INTERNALS__.invoke('qa_sleep', { ms }); // webview timers stop when the window is covered
  const log = document.createElement('pre');
  log.style.cssText = 'position:fixed;left:270px;bottom:10px;z-index:99999;background:#000c;color:#7f7;font:12px monospace;padding:8px;max-width:680px;white-space:pre-wrap;border-radius:8px';
  document.body.append(log);
  const say = (ok, msg) => { const line = (ok ? 'PASS ' : 'FAIL ') + msg; log.textContent += line + '\n'; window.__TAURI_INTERNALS__.invoke('qa_log', { line }); };
  const $ = s => document.querySelector(s);
  // Each check retries for up to ~6 s: macOS slows a covered or sleeping app, so fixed waits are not enough.
  const ok = async (check, message) => { let pass = false; for (let i = 0; i < 60 && !(pass = !!(await check())); i++) await wait(100); say(pass, message()); };
  const set = (el, v) => { el.value = v; el.dispatchEvent(new Event('input', { bubbles: true })); };
  const card = name => [...document.querySelectorAll('.wish')].find(c => c.querySelector('h3')?.textContent === name);
  const btn = (root, text) => [...root.querySelectorAll('button')].find(b => b.textContent.includes(text));
  const saved = name => card(name)?.querySelector('.bar + div .muted')?.textContent;
  const toastText = () => $('.toast.show')?.textContent.trim() ?? '';
  // The toast's button while it is visible, otherwise ⌘Z (the toast hides after 6 s).
  const undo = async () => { const b = $('.toast.show button'); if (b) b.click(); else document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'z', metaKey: true, bubbles: true })); await wait(1000); };
  const reserve = async (name, amount) => { btn(card(name), 'Apartar más').click(); await wait(300); const f = card(name).querySelector('form.reserve'); set(f.querySelector('input'), amount); f.requestSubmit(); await wait(1500); };
  try {
    await wait(1200); location.hash = 'deseos'; await wait(1200);
    await ok(() => document.querySelectorAll('.wish h3').length === 3, () => `3 wishes listed: ${[...document.querySelectorAll('.wish h3')].map(h => h.textContent).join(', ')}`);
    await ok(() => card('Audífonos inalámbricos')?.textContent.includes('Listo para comprar'), () => 'fully saved wish says "Listo para comprar"');
    await ok(() => card('PlayStation 5')?.querySelector('.chip.warn')?.textContent.includes('Falta presupuesto'), () => `PS5 verdict: "${card('PlayStation 5')?.querySelector('.chip')?.textContent}"`);

    await reserve('Silla ergonómica', '100');
    await ok(() => saved('Silla ergonómica')?.includes('$220.00'), () => `Apartar 100 on Silla: "${saved('Silla ergonómica')}"`);
    await undo();
    await ok(() => saved('Silla ergonómica')?.includes('$120.00'), () => `undo restores "${saved('Silla ergonómica')}"`);
    await reserve('PlayStation 5', '50');
    await ok(() => toastText().includes('No hay suficiente dinero libre'), () => `over-reserve shows the core rule in Spanish: "${toastText()}"`);
    btn(card('Silla ergonómica'), 'Liberar').click(); await wait(1500);
    await ok(() => saved('Silla ergonómica')?.includes('$0.00'), () => 'Liberar sets it back to $0.00');
    await undo();

    btn(card('Audífonos inalámbricos'), 'Lo compré').click(); await wait(1500);
    await ok(() => !card('Audífonos inalámbricos'), () => 'Lo compré archives the wish');
    location.hash = 'movimientos'; await wait(1000);
    await ok(() => [...document.querySelectorAll('tr.tx')].some(r => r.textContent.includes('Audífonos') && r.textContent.includes('-$150.00')), () => 'and records the -$150.00 expense');
    location.hash = 'deseos'; await wait(1000);
    await undo();
    await ok(() => !!card('Audífonos inalámbricos'), () => 'undo brings the wish back');
    location.hash = 'movimientos'; await wait(1000);
    await ok(() => ![...document.querySelectorAll('tr.tx')].some(r => r.textContent.includes('Audífonos')), () => 'and removes the expense');
    location.hash = 'deseos'; await wait(1000);

    btn($('.topbar'), 'Nueva meta').click(); await wait(500);
    const d = $('dialog[open]');
    set(d.querySelector('input[placeholder="PlayStation 5"]'), 'QA Bici');
    set(d.querySelector('input[placeholder="11,000.00"]'), '1,2');
    d.querySelector('button[value="next"]').click(); await wait(300);
    await ok(() => d.textContent.includes('Escribe un precio válido') && d.textContent.includes('Paso 1 de 2'), () => 'invalid price "1,2" stays on step 1 with an error');
    set(d.querySelector('input[placeholder="11,000.00"]'), '11,000');
    d.querySelector('button[value="next"]').click(); await wait(300);
    await ok(() => d.textContent.includes('Paso 2 de 2') && d.querySelector('.review')?.textContent.includes('$11,000.00'), () => 'step 2 shows the price read as $11,000.00');
    d.querySelector('button[value="next"]').click(); await wait(1500);
    await ok(() => !$('dialog[open]') && !!card('QA Bici'), () => 'Guardar deseo adds the card');

    card('QA Bici').querySelector('button[aria-label^="Archivar"]').click(); await wait(1500);
    await ok(() => !card('QA Bici'), () => 'Archivar hides it');
    btn($('.topbar'), 'Archivados').click(); await wait(1000);
    await ok(() => !!card('QA Bici') && card('QA Bici').textContent.includes('Archivado'), () => 'it appears under Archivados');
    btn($('.topbar'), 'Ver activos').click(); await wait(800);
    await undo();
    await ok(() => !!card('QA Bici'), () => 'undo restores the archived wish');

    // A trip with a date ~6 months out: the card shows its type and the monthly amount to set aside.
    btn($('.topbar'), 'Nueva meta').click(); await wait(500);
    const t = $('dialog[open]');
    btn(t, 'Viaje').click(); await wait(200);
    await ok(() => !!t.querySelector('input[placeholder="Viaje a Cancún"]'), () => 'choosing Viaje changes the example name');
    set(t.querySelector('input[placeholder="Viaje a Cancún"]'), 'QA Cancún');
    set(t.querySelector('input[placeholder="25,000.00"]'), '8,400');
    const now = new Date(), target = new Date(now.getFullYear(), now.getMonth() + 6, 1);
    const iso = `${target.getFullYear()}-${String(target.getMonth() + 1).padStart(2, '0')}-01`;
    set(t.querySelector('input[type="date"]'), iso);
    t.querySelector('button[value="next"]').click(); await wait(300);
    await ok(() => t.querySelector('.review')?.textContent.includes('Viaje'), () => 'the review names the type');
    t.querySelector('button[value="next"]').click(); await wait(1500);
    await ok(() => card('QA Cancún')?.textContent.includes('Viaje') && /Aparta \$[\d,.]+ al mes · \d+ meses/.test(card('QA Cancún')?.querySelector('.plan')?.textContent ?? ''),
      () => `the trip card shows its plan: "${card('QA Cancún')?.querySelector('.plan')?.textContent}"`);

    // Line items: the price is their sum, and paying one counts as progress.
    btn($('.topbar'), 'Nueva meta').click(); await wait(500);
    const li = $('dialog[open]');
    btn(li, 'Viaje').click(); await wait(200);
    set(li.querySelector('input[placeholder="Viaje a Cancún"]'), 'QA Mérida');
    btn(li, '+ Vuelo').click(); await wait(100); btn(li, '+ Hotel').click(); await wait(200);
    set(li.querySelector('input[aria-label="Monto de la partida 1"]'), '3,000');
    set(li.querySelector('input[aria-label="Monto de la partida 2"]'), '2,000');
    await wait(200);
    await ok(() => [...li.querySelectorAll('input[readonly]')].some(i => i.value === '$5,000.00'), () => 'two items add up to $5,000.00');
    li.querySelector('button[value="next"]').click(); await wait(300);
    await ok(() => li.querySelector('.review')?.textContent.includes('Vuelo $3,000.00'), () => 'the review lists the items');
    li.querySelector('button[value="next"]').click(); await wait(1500);
    await ok(() => card('QA Mérida')?.querySelectorAll('.items li').length === 2, () => 'the card lists both items');
    card('QA Mérida').querySelector('.items input[type="checkbox"]').click(); await wait(1500);
    await ok(() => saved('QA Mérida')?.includes('Pagado $3,000.00'), () => `paying the flight counts as progress: "${saved('QA Mérida')}"`);
    await undo();
    await ok(() => !saved('QA Mérida')?.includes('Pagado'), () => 'undo unmarks it');
  } catch (e) { say(false, 'exception: ' + e); }
  window.__TAURI_INTERNALS__.invoke('qa_log', { line: 'DONE' });
})();
