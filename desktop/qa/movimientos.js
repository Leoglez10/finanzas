// Movimientos: add with validation, undo toast, inline edit, delete + undo, search, filters.
(async () => {
  const wait = ms => window.__TAURI_INTERNALS__.invoke('qa_sleep', { ms }); // webview timers stop when the window is covered
  const log = document.createElement('pre');
  log.style.cssText = 'position:fixed;left:270px;bottom:10px;z-index:99999;background:#000c;color:#7f7;font:12px monospace;padding:8px;max-width:640px;white-space:pre-wrap;border-radius:8px';
  document.body.append(log);
  const say = (ok, msg) => { const line = (ok ? 'PASS ' : 'FAIL ') + msg; log.textContent += line + '\n'; window.__TAURI_INTERNALS__.invoke('qa_log', { line }); };
  const $ = s => document.querySelector(s);
  // Each check retries for up to ~6 s: macOS slows a covered or sleeping app, so fixed waits are not enough.
  const ok = async (check, message) => { let pass = false; for (let i = 0; i < 60 && !(pass = !!(await check())); i++) await wait(100); say(pass, message()); };
  const byText = (sel, text) => [...document.querySelectorAll(sel)].find(e => e.textContent.trim().includes(text));
  const set = (el, value, ev = 'input') => { el.value = value; el.dispatchEvent(new Event(ev, { bubbles: true })); };
  const rowsOf = name => [...document.querySelectorAll('tr.tx')].filter(r => r.textContent.includes(name));
  try {
    await wait(1200);
    location.hash = 'movimientos'; await wait(900);
    await ok(() => document.querySelectorAll('tr.tx').length > 0, () => `ledger shows ${document.querySelectorAll('tr.tx').length} rows grouped by day (${document.querySelectorAll('th.day').length} days)`);

    byText('.topbar button', 'Gasto').click(); await wait(400);
    const dlg = $('dialog[open]');
    await ok(() => !!dlg, () => 'Gasto opens the movement dialog');
    set(dlg.querySelector('input[aria-label="Monto"]'), '1,2');
    dlg.querySelector('button[value="save"]').click(); await wait(300);
    await ok(() => !!dlg.querySelector('.err'), () => `invalid amount "1,2" is rejected: ${dlg.querySelector('.err')?.textContent}`);
    set(dlg.querySelector('input[aria-label="Monto"]'), '120');
    const payee = dlg.querySelector('input[list="payee-list"]'); set(payee, 'QA Taquería'); set(payee, 'QA Taquería', 'change');
    await wait(300);
    dlg.querySelector('button[value="save"]').click(); await wait(1200);
    await ok(() => !$('dialog[open]'), () => 'valid gasto saves and closes');
    let added = [];
    await ok(() => (added = rowsOf('QA Taquería')).length === 1 && added[0].textContent.includes('-$120.00'), () => `new row appears with -$120.00`);
    await ok(() => $('.toast.show')?.textContent.includes('Deshacer'), () => `toast offers undo: "${$('.toast.show')?.textContent.trim()}"`);

    added[0].querySelector('td.amt').dispatchEvent(new MouseEvent('dblclick', { bubbles: true })); await wait(300);
    let inline = null;
    await ok(() => !!(inline = rowsOf('QA Taquería')[0]?.querySelector('td.amt input')), () => 'double click opens inline amount editor');
    set(inline, '-150'); inline.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })); await wait(1200);
    await ok(() => rowsOf('QA Taquería')[0]?.textContent.includes('-$150.00'), () => 'inline edit saves -$150.00');

    rowsOf('QA Taquería')[0].querySelector('button[aria-label^="Eliminar"]').click(); await wait(1200);
    await ok(() => rowsOf('QA Taquería').length === 0, () => 'delete removes the row without a confirmation dialog');
    $('.toast.show button').click(); await wait(1500);
    await ok(() => rowsOf('QA Taquería').length === 1 && rowsOf('QA Taquería')[0].textContent.includes('-$150.00'), () => 'Deshacer restores the deleted row');

    set($('input[type=search]'), 'taquer'); await wait(300);
    await ok(() => document.querySelectorAll('tr.tx').length === 1, () => `search "taquer" (accent-insensitive) leaves ${document.querySelectorAll('tr.tx').length} row`);
    set($('input[type=search]'), ''); await wait(200);
    byText('.seg button', 'Ingresos').click(); await wait(300);
    await ok(() => [...document.querySelectorAll('tr.tx td.amt')].every(td => td.textContent.trim().startsWith('+')), () => 'Ingresos filter shows only positive amounts');
    byText('.seg button', 'Todos').click();
    location.hash = 'movimientos?mes=' + (new Date().getFullYear() * 100 + new Date().getMonth()); await wait(800);
    await ok(() => !!byText('.chip.brand', ''), () => `month filter chip from route: "${$('.toolbar .chip.brand')?.textContent.trim()}"`);
    location.hash = 'movimientos'; await wait(600);
    document.title = 'QA-DONE';
  } catch (e) { say(false, 'exception: ' + e); document.title = 'QA-DONE'; }
  window.__TAURI_INTERNALS__.invoke('qa_log', { line: 'DONE' });
})();
