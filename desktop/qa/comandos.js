// T7: ⌘K palette (navigate, free-text register + undo, reserve for a wish, theme) and menu events.
// Native accelerators and the global ⌥⇧F need real key presses and are not covered here.
(async () => {
  const wait = ms => window.__TAURI_INTERNALS__.invoke('qa_sleep', { ms }); // webview timers stop when the window is covered
  const inv = (c, a) => window.__TAURI_INTERNALS__.invoke(c, a);
  const log = document.createElement('pre');
  log.style.cssText = 'position:fixed;left:270px;bottom:10px;z-index:99999;background:#000c;color:#7f7;font:12px monospace;padding:8px;max-width:680px;white-space:pre-wrap;border-radius:8px';
  document.body.append(log);
  const say = (ok, msg) => { const line = (ok ? 'PASS ' : 'FAIL ') + msg; log.textContent += line + '\n'; inv('qa_log', { line }); };
  const $ = s => document.querySelector(s);
  const ok = async (check, message) => { let pass = false; for (let i = 0; i < 60 && !(pass = !!(await check())); i++) await wait(100); say(pass, message()); };
  const menu = id => inv('plugin:event|emit', { event: 'menu', payload: id });
  const type = async text => { const i = $('dialog.palette[open] input'); i.value = text; i.dispatchEvent(new Event('input', { bubbles: true })); await wait(300); };
  const enter = () => $('dialog.palette[open] input').dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
  const first = () => $('dialog.palette[open] li[aria-selected="true"]')?.textContent.trim() ?? '';
  const openPalette = async () => { $('.sidebar .cmdk').click(); await ok(() => $('dialog.palette[open]'), () => 'palette opens from the sidebar button'); await wait(800); };
  try {
    await wait(1200);
    await menu('go-presupuesto');
    await ok(() => location.hash === '#presupuesto', () => `menu event "go-presupuesto" navigates (${location.hash})`);
    await menu('new-gasto');
    await ok(() => $('dialog[open] input[aria-label="Monto"]'), () => 'menu event "new-gasto" opens the movement dialog');
    $('dialog[open] button[value="cancel"]').click(); await wait(300);

    await openPalette();
    await type('netflix');
    await ok(() => first().includes('Movimientos de Netflix'), () => `"netflix" offers: "${first()}"`);
    enter();
    await ok(() => location.hash.startsWith('#movimientos?q=') && [...document.querySelectorAll('tr.tx')].length > 0 && [...document.querySelectorAll('tr.tx')].every(r => r.textContent.includes('Netflix')), () => `choosing it filters Movimientos (${document.querySelectorAll('tr.tx').length} rows)`);

    await openPalette();
    await type('uber 120 ayer');
    await ok(() => first().includes('Registrar gasto: Uber -$120.00'), () => `"uber 120 ayer" offers: "${first()}"`);
    enter();
    await ok(() => $('.toast.show')?.textContent.includes('Guardado: Uber -$120.00'), () => `saved with undo toast: "${$('.toast.show')?.textContent.trim()}"`);
    const [y, m, d] = (await inv('today')).split('-').map(Number), prev = new Date(y, m - 1, d - 1);
    const yesterday = `${prev.getFullYear()}-${String(prev.getMonth() + 1).padStart(2, '0')}-${String(prev.getDate()).padStart(2, '0')}`; // local, not UTC
    await ok(async () => (await inv('entries', { filter: { search: 'Uber' } })).rows.some(r => r.amount === -12000 && r.date === yesterday), () => `the movement is -$120.00 dated ${yesterday}`);
    document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'z', metaKey: true, bubbles: true }));
    await ok(async () => !(await inv('entries', { filter: { search: 'Uber' } })).rows.some(r => r.amount === -12000), () => '⌘Z undoes it');

    await openPalette();
    await type('apartar 100 silla');
    await ok(() => first().includes('Apartar $100.00 para Silla ergonómica'), () => `"apartar 100 silla" offers: "${first()}"`);
    enter();
    await ok(async () => (await inv('wishes', { archived: false })).find(w => w.wish.name === 'Silla ergonómica')?.wish.reserved === 22000, () => 'it reserves $100 more (120 → 220)');

    await openPalette();
    await type('tema arena');
    await ok(() => first().includes('Tema: Arena'), () => `"tema arena" offers: "${first()}"`);
    enter();
    await ok(() => document.documentElement.dataset.palette === 'arena', () => 'and applies the theme');

    await openPalette();
    await type('zzzz');
    await ok(() => $('dialog.palette[open] .empty'), () => 'no match shows a hint');
    $('dialog.palette[open]').close();
    location.hash = 'ajustes/atajos';
    await ok(() => $('.keys')?.textContent.includes('⌘K') && $('.keys')?.textContent.includes('⌥⇧F'), () => 'Atajos lists ⌘K and ⌥⇧F from the registry');
    await ok(async () => { await inv('show_quick'); await inv('hide_quick'); return true; }, () => 'show_quick / hide_quick commands answer');
  } catch (e) { say(false, 'exception: ' + e); }
  inv('qa_log', { line: 'DONE' });
})();
