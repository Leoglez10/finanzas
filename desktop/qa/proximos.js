// Próximos pagos: register + undo, skip + undo, create/edit/delete schedule + undo, overdue badge.
(async () => {
  const wait = ms => window.__TAURI_INTERNALS__.invoke('qa_sleep', { ms }); // webview timers stop when the window is covered
  const log = document.createElement('pre');
  log.style.cssText = 'position:fixed;left:270px;bottom:10px;z-index:99999;background:#000c;color:#7f7;font:12px monospace;padding:8px;max-width:680px;white-space:pre-wrap;border-radius:8px';
  document.body.append(log);
  const say = (ok, msg) => { const line = (ok ? 'PASS ' : 'FAIL ') + msg; log.textContent += line + '\n'; window.__TAURI_INTERNALS__.invoke('qa_log', { line }); };
  const $ = s => document.querySelector(s);
  // Each check retries for up to ~6 s: macOS slows a covered or sleeping app, so fixed waits are not enough.
  const ok = async (check, message) => { let pass = false; for (let i = 0; i < 60 && !(pass = !!(await check())); i++) await wait(100); say(pass, message()); };
  const set = (el, v, ev = 'input') => { el.value = v; el.dispatchEvent(new Event(ev, { bubbles: true })); };
  // Upcoming rows only (they carry a date tile); the schedules list has rows with the same names.
  const rowOf = name => [...document.querySelectorAll('.card .row')].find(r => r.querySelector('.date') && r.querySelector('.title')?.textContent.trim() === name);
  const badge = () => $('.sidebar .count')?.textContent.trim() ?? '0';
  // The toast's button while it is visible, otherwise ⌘Z (the toast hides after 6 s).
  const undo = async () => { const b = $('.toast.show button'); if (b) b.click(); else document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'z', metaKey: true, bubbles: true })); await wait(1000); };
  try {
    await wait(1200); location.hash = 'proximos'; await wait(1200);
    await ok(() => badge() === '1' && !!rowOf('Renta')?.querySelector('.chip.bad'), () => `overdue Renta listed, sidebar badge ${badge()}`);
    const rentDate = rowOf('Renta').querySelector('.date').textContent;
    rowOf('Renta').querySelector('button.primary').click(); await wait(1500);
    await ok(() => badge() === '0', () => `Registrar clears the overdue badge (${badge()})`);
    await ok(() => $('.toast.show')?.textContent.includes('Registrado: Renta'), () => `toast: "${$('.toast.show')?.textContent.trim()}"`);
    location.hash = 'movimientos'; await wait(900);
    await ok(() => [...document.querySelectorAll('tr.tx')].some(r => r.textContent.includes('Renta') && r.textContent.includes('-$800.00')), () => 'the payment appears in Movimientos (-$800.00)');
    location.hash = 'proximos'; await wait(900);
    await undo();
    await ok(() => badge() === '1' && rowOf('Renta')?.querySelector('.date').textContent === rentDate, () => 'Deshacer brings the overdue payment back with its date');
    location.hash = 'movimientos'; await wait(900);
    await ok(() => ![...document.querySelectorAll('tr.tx')].some(r => r.textContent.includes('Renta') && r.textContent.includes('-$800.00')), () => 'and removes the registered movement');
    location.hash = 'proximos'; await wait(900);

    const netflixDate = rowOf('Netflix').querySelector('.date').textContent;
    rowOf('Netflix').querySelector('button.ghost').click(); await wait(1500);
    await ok(() => !rowOf('Netflix') || rowOf('Netflix').querySelector('.date').textContent !== netflixDate, () => `Saltar moves Netflix past ${netflixDate}`);
    await undo();
    await ok(() => rowOf('Netflix')?.querySelector('.date').textContent === netflixDate, () => 'Deshacer restores the skipped date');

    [...document.querySelectorAll('.topbar button')].find(b => b.textContent.includes('Programar')).click(); await wait(500);
    const dlg = $('dialog[open]');
    dlg.querySelector('button[value="save"]').click(); await wait(300);
    await ok(() => dlg.querySelector('.err')?.textContent.includes('nombre'), () => `empty name rejected: "${dlg.querySelector('.err')?.textContent}"`);
    set(dlg.querySelector('input[placeholder="Renta"]'), 'QA Gimnasio');
    set(dlg.querySelector('input[placeholder="1,250.50"]'), '350');
    dlg.querySelector('button[value="save"]').click(); await wait(1500);
    const programmed = () => [...document.querySelectorAll('.row')].find(r => r.querySelector('button[aria-label^="Editar"]') && r.textContent.includes('QA Gimnasio'));
    await ok(() => !$('dialog[open]') && programmed()?.textContent.includes('-$350.00'), () => `new schedule listed: "${programmed()?.querySelector('.meta')?.textContent}"`);
    programmed().querySelector('button[aria-label^="Editar"]').click(); await wait(500);
    set($('dialog[open] input[placeholder="1,250.50"]'), '400'); $('dialog[open] button[value="save"]').click(); await wait(1500);
    await ok(() => programmed()?.textContent.includes('-$400.00'), () => 'editing the amount saves -$400.00');
    programmed().querySelector('button[aria-label^="Eliminar"]').click(); await wait(1500);
    await ok(() => !programmed(), () => 'Eliminar removes it without a confirmation dialog');
    await undo();
    await ok(() => programmed()?.textContent.includes('-$400.00'), () => 'Deshacer brings the deleted schedule back');
  } catch (e) { say(false, 'exception: ' + e); }
  window.__TAURI_INTERNALS__.invoke('qa_log', { line: 'DONE' });
})();
