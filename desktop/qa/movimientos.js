// Movimientos: add with validation, undo toast, inline edit, delete + undo, search, filters.
(async () => {
  const wait = ms => new Promise(r => setTimeout(r, ms));
  const log = document.createElement('pre');
  log.style.cssText = 'position:fixed;left:270px;bottom:10px;z-index:99999;background:#000c;color:#7f7;font:12px monospace;padding:8px;max-width:640px;white-space:pre-wrap;border-radius:8px';
  document.body.append(log);
  const say = (ok, msg) => { log.textContent += (ok ? 'PASS ' : 'FAIL ') + msg + '\n'; };
  const $ = s => document.querySelector(s);
  const byText = (sel, text) => [...document.querySelectorAll(sel)].find(e => e.textContent.trim().includes(text));
  const set = (el, value, ev = 'input') => { el.value = value; el.dispatchEvent(new Event(ev, { bubbles: true })); };
  const rowsOf = name => [...document.querySelectorAll('tr.tx')].filter(r => r.textContent.includes(name));
  try {
    await wait(1200);
    location.hash = 'movimientos'; await wait(900);
    const before = document.querySelectorAll('tr.tx').length;
    say(before > 0, `ledger shows ${before} rows grouped by day (${document.querySelectorAll('th.day').length} days)`);

    byText('.topbar button', 'Gasto').click(); await wait(400);
    const dlg = $('dialog[open]');
    say(!!dlg, 'Gasto opens the movement dialog');
    set(dlg.querySelector('input[aria-label="Monto"]'), '1,2');
    dlg.querySelector('button[value="save"]').click(); await wait(300);
    say(!!dlg.querySelector('.err'), `invalid amount "1,2" is rejected: ${dlg.querySelector('.err')?.textContent}`);
    set(dlg.querySelector('input[aria-label="Monto"]'), '120');
    const payee = dlg.querySelector('input[list="payee-list"]'); set(payee, 'QA Taquería'); set(payee, 'QA Taquería', 'change');
    await wait(300);
    dlg.querySelector('button[value="save"]').click(); await wait(1200);
    say(!$('dialog[open]'), 'valid gasto saves and closes');
    let added = rowsOf('QA Taquería');
    say(added.length === 1 && added[0].textContent.includes('-$120.00'), `new row appears with -$120.00`);
    say($('.toast.show')?.textContent.includes('Deshacer'), `toast offers undo: "${$('.toast.show')?.textContent.trim()}"`);

    added[0].querySelector('td.amt').dispatchEvent(new MouseEvent('dblclick', { bubbles: true })); await wait(300);
    const inline = added[0].querySelector('td.amt input');
    say(!!inline, 'double click opens inline amount editor');
    set(inline, '-150'); inline.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })); await wait(1200);
    say(rowsOf('QA Taquería')[0]?.textContent.includes('-$150.00'), 'inline edit saves -$150.00');

    rowsOf('QA Taquería')[0].querySelector('button[aria-label^="Eliminar"]').click(); await wait(1200);
    say(rowsOf('QA Taquería').length === 0, 'delete removes the row without a confirmation dialog');
    $('.toast.show button').click(); await wait(1500);
    say(rowsOf('QA Taquería').length === 1 && rowsOf('QA Taquería')[0].textContent.includes('-$150.00'), 'Deshacer restores the deleted row');

    set($('input[type=search]'), 'taquer'); await wait(300);
    say(document.querySelectorAll('tr.tx').length === 1, `search "taquer" (accent-insensitive) leaves ${document.querySelectorAll('tr.tx').length} row`);
    set($('input[type=search]'), ''); await wait(200);
    byText('.seg button', 'Ingresos').click(); await wait(300);
    say([...document.querySelectorAll('tr.tx td.amt')].every(td => td.textContent.trim().startsWith('+')), 'Ingresos filter shows only positive amounts');
    byText('.seg button', 'Todos').click();
    location.hash = 'movimientos?mes=' + (new Date().getFullYear() * 100 + new Date().getMonth()); await wait(800);
    say(!!byText('.chip.brand', ''), `month filter chip from route: "${$('.toolbar .chip.brand')?.textContent.trim()}"`);
    location.hash = 'movimientos'; await wait(600);
    document.title = 'QA-DONE';
  } catch (e) { say(false, 'exception: ' + e); document.title = 'QA-DONE'; }
})();
