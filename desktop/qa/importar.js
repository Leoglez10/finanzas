// T8: CSV preview (new / duplicate / error rows), account choice, all-or-nothing import, undo, file reading rules.
(async () => {
  const wait = ms => window.__TAURI_INTERNALS__.invoke('qa_sleep', { ms }); // webview timers stop when the window is covered
  const inv = (c, a) => window.__TAURI_INTERNALS__.invoke(c, a);
  const log = document.createElement('pre');
  log.style.cssText = 'position:fixed;left:270px;bottom:10px;z-index:99999;background:#000c;color:#7f7;font:12px monospace;padding:8px;max-width:680px;white-space:pre-wrap;border-radius:8px';
  document.body.append(log);
  const say = (ok, msg) => { const line = (ok ? 'PASS ' : 'FAIL ') + msg; log.textContent += line + '\n'; inv('qa_log', { line }); };
  const $ = s => document.querySelector(s);
  const ok = async (check, message) => { let pass = false; for (let i = 0; i < 60 && !(pass = !!(await check())); i++) await wait(100); say(pass, message()); };
  const status = () => [...document.querySelectorAll('dialog.import[open] tbody tr')].map(tr => tr.lastElementChild.textContent.trim());
  const go = $ => $('dialog.import[open] button[value="import"]');
  try {
    await wait(1200);
    const { openImport } = window.__finanzas; // dev-only hook from App.svelte
    const [y, m] = (await inv('today')).split('-');
    const day = d => `${d}/${m}/${y}`;
    // Netflix on the 5th is already in the demo data (Cuenta principal, -350.00).
    const text = `Fecha,Comercio,Monto,Categoría\n${day('05')},Netflix,-350.00,\n${day('08')},Gasolinera Shell,"-820.50",\n${day('07')},Oxxo,-64.00,\n31/13/${y},Fecha rota,-10,\n${day('06')},Reembolso seguro,"1,200.00",`;
    openImport({ name: 'banco.csv', text });
    await ok(() => status().length === 5, () => `preview shows 5 rows: ${status().join(' | ')}`);
    await ok(() => status()[0] === 'Duplicado' && status()[3].includes('Fecha no válida') && status().filter(s => s === 'Nuevo').length === 3, () => 'duplicate and broken rows are marked, 3 new');
    await ok(() => go($)?.textContent.includes('Importar 3 movimientos'), () => `button: "${go($)?.textContent.trim()}"`);
    await ok(() => [...document.querySelectorAll('dialog.import[open] tbody tr')][1].textContent.includes('Transporte'), () => 'Gasolinera gets Transporte from the keyword list');

    const select = $('dialog.import[open] select'), tarjeta = [...select.options].find(o => o.textContent === 'Tarjeta');
    select.value = tarjeta.value; select.dispatchEvent(new Event('change', { bubbles: true }));
    await ok(() => status()[0] === 'Nuevo' && go($)?.textContent.includes('Importar 4'), () => 'in another account Netflix is no longer a duplicate (4 to import)');
    select.value = [...select.options].find(o => o.textContent === 'Cuenta principal').value; select.dispatchEvent(new Event('change', { bubbles: true }));
    await ok(() => go($)?.textContent.includes('Importar 3'), () => 'back to Cuenta principal: 3 to import');

    const before = (await inv('entries', { filter: {} })).rows.length;
    go($).click();
    await ok(async () => !$('dialog.import[open]') && (await inv('entries', { filter: {} })).rows.length === before + 3, () => 'Importar adds exactly 3 movements');
    await ok(async () => (await inv('entries', { filter: { search: 'Reembolso' } })).rows.some(r => r.amount === 120000 && r.notes === 'Importado'), () => 'the income row keeps its sign and is noted "Importado"');
    await ok(() => $('.toast.show')?.textContent.includes('Importaste 3 movimientos'), () => `toast: "${$('.toast.show')?.textContent.trim()}"`);
    document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'z', metaKey: true, bubbles: true }));
    await ok(async () => (await inv('entries', { filter: {} })).rows.length === before, () => '⌘Z removes the whole import');

    let failed = '';
    try { await inv('import_entries', { entries: [{ date: `${y}-${m}-01`, amount: -100, accountId: 1, payee: 'QA ok', categoryId: null, notes: '' }, { date: `${y}-${m}-01`, amount: -100, accountId: 999, payee: 'QA bad', categoryId: null, notes: '' }] }); } catch (e) { failed = String(e); }
    await ok(async () => failed && !(await inv('entries', { filter: { search: 'QA ok' } })).rows.length, () => `a failing row rolls back the whole batch (${failed})`);

    const file = await inv('read_csv', { path: '__QA__/fixtures/banco.csv' });
    await ok(() => file.name === 'banco.csv' && file.text.includes('Gasolinera Shell'), () => 'read_csv reads a .csv file');
    let rejected = ''; try { await inv('read_csv', { path: '__QA__/fixtures/notas.txt' }); } catch (e) { rejected = String(e); }
    await ok(() => rejected.includes('Solo puedo importar archivos .csv'), () => `other files are refused: "${rejected}"`);
    openImport(file);
    await ok(() => status().length === 2 && document.querySelector('dialog.import[open]').textContent.includes('-$820.50') && document.querySelector('dialog.import[open]').textContent.includes('+$1,200.00'), () => 'a bank file with ";" and Cargo/Abono columns previews -$820.50 and +$1,200.00');
    $('dialog.import[open]').close();
  } catch (e) { say(false, 'exception: ' + e); }
  inv('qa_log', { line: 'DONE' });
})();
