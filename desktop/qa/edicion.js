// Easier editing: names open their edit dialog, right-click menus, clickable cards and links, ⌘K "editar …".
(async () => {
  const wait = ms => window.__TAURI_INTERNALS__.invoke('qa_sleep', { ms }); // webview timers stop when the window is covered
  const log = document.createElement('pre');
  log.style.cssText = 'position:fixed;left:270px;bottom:10px;z-index:99999;background:#000c;color:#7f7;font:12px monospace;padding:8px;max-width:680px;white-space:pre-wrap;border-radius:8px';
  document.body.append(log);
  const say = (ok, msg) => { const line = (ok ? 'PASS ' : 'FAIL ') + msg; log.textContent += line + '\n'; window.__TAURI_INTERNALS__.invoke('qa_log', { line }); };
  const $ = s => document.querySelector(s), $$ = s => [...document.querySelectorAll(s)];
  const ok = async (check, message) => { let pass = false; for (let i = 0; i < 60 && !(pass = !!(await check())); i++) await wait(100); say(pass, message()); };
  const close = async () => { $('dialog[open]')?.close(); await wait(300); };
  let qa; // window.__finanzas appears once the app mounts
  const rightClick = el => { qa.lastMenu = null; el.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true })); };
  const menu = () => (qa.lastMenu ?? []).map(x => x ? x[0] : '—');
  const pick = label => qa.lastMenu.find(x => x && x[0] === label)[1]();
  const options = () => $$('dialog.palette [role="option"]');
  try {
    await wait(1200);
    qa = window.__finanzas; qa.capture = true; // right-click menus are recorded instead of shown

    // Próximos: name, right-click, card, account link.
    location.hash = 'proximos'; await wait(1200);
    const upcoming = name => $$('.card .row').find(r => r.querySelector('.date') && r.querySelector('.title')?.textContent.trim() === name);
    upcoming('Renta').querySelector('.title button').click();
    await ok(() => $('dialog[open] input[autofocus]')?.value === 'Renta', () => 'Próximos: the name opens the schedule');
    await close();
    rightClick(upcoming('Renta'));
    await ok(() => menu().join('|') === 'Registrar|Saltar esta vez|—|Editar pago programado…|Abrir Cuenta principal', () => `Próximos: right-click menu ${menu().join(' | ')}`);
    pick('Editar pago programado…');
    await ok(() => $('dialog[open] input[autofocus]')?.value === 'Renta', () => 'the menu opens the schedule');
    await close();
    $$('.kpis .kpi.link')[0].click();
    await ok(() => Math.abs($('#vencidos').getBoundingClientRect().top) < 200, () => 'the Vencido card scrolls to the overdue payments');
    upcoming('Renta').querySelector('.meta .text-link').click();
    await ok(() => location.hash.startsWith('#cuenta-'), () => `the account name opens the account (${location.hash})`);

    // Movimientos: name, right-click menu (duplicate, filter), cards filter the list, account link.
    location.hash = 'movimientos'; await wait(1200);
    const netflix = () => $$('tr.tx').find(r => r.textContent.includes('Netflix'));
    netflix().querySelector('button.edit-name').click();
    await ok(() => $('#tx-title')?.textContent === 'Editar movimiento', () => 'Movimientos: the name opens the movement');
    await close();
    rightClick(netflix());
    await ok(() => menu()[0] === 'Editar…' && menu().includes('Duplicar con fecha de hoy…') && menu().at(-1) === 'Eliminar' && menu().at(-2) === '—', () => `Movimientos: right-click menu ${menu().join(' | ')}`);
    pick('Duplicar con fecha de hoy…');
    await ok(() => $('#tx-title')?.textContent === 'Nuevo movimiento' && $$('dialog[open] input').some(i => i.value === 'Netflix'), () => 'Duplicar opens a new movement with the same payee');
    await close();
    rightClick(netflix());
    const filter = menu().find(l => l.startsWith('Ver solo'));
    pick(filter);
    await ok(() => location.hash.includes('categoria='), () => `"${filter}" filters by its category (${location.hash})`);
    location.hash = 'movimientos'; await wait(900);
    const rows = () => $$('tr.tx').length;
    const all = rows();
    $$('.kpis .kpi.link')[2].click();
    await ok(() => rows() > 0 && rows() < all && $$('tr.tx .amt').every(td => !td.classList.contains('pos')), () => `the Salidas card shows only outflows (${rows()} of ${all})`);
    $$('.kpis .kpi.link')[0].click();
    await ok(() => rows() === all, () => 'the balance card shows everything again');
    $('tr.tx .text-link').click();
    await ok(() => location.hash.startsWith('#cuenta-'), () => 'the account column opens the account');

    // Resumen: the Gastos card opens the period's expenses.
    location.hash = 'resumen'; await wait(1500);
    $$('.kpis .kpi.link').find(k => k.textContent.includes('Gastos')).click();
    await ok(() => location.hash.includes('tipo=gastos') && $('[aria-pressed="true"]')?.textContent.includes('Gastos'), () => `Resumen: the Gastos card opens expenses (${location.hash})`);

    // Deseos: right-click.
    location.hash = 'deseos'; await wait(1200);
    rightClick($('.card.wish'));
    await ok(() => menu().join('|') === 'Editar…|—|Archivar', () => `Deseos: right-click menu ${menu().join(' | ')}`);

    // ⌘K: "editar renta" lists the schedule first and opens it.
    qa.runCommand('palette'); await wait(400);
    const input = $('dialog.palette input');
    input.value = 'editar renta'; input.dispatchEvent(new Event('input', { bubbles: true }));
    await ok(() => options()[0]?.textContent.includes('Editar pago: Renta'), () => `⌘K "editar renta" first result: "${options()[0]?.textContent.trim()}"`);
    await ok(() => options().some(o => o.textContent.includes('Editar movimiento: Renta')), () => '⌘K also finds Renta movements');
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    await ok(() => $('dialog[open] input[autofocus]')?.value === 'Renta', () => 'Enter opens the schedule to edit');
    await close();
  } catch (e) { say(false, 'exception: ' + e); }
  window.__TAURI_INTERNALS__.invoke('qa_log', { line: 'DONE' });
})();
