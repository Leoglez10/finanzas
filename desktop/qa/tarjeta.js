// Card data is easy to reach: account page button, sidebar right-click, Deudas "Datos" button and terms line.
(async () => {
  const wait = ms => window.__TAURI_INTERNALS__.invoke('qa_sleep', { ms });
  const invoke = (cmd, args) => window.__TAURI_INTERNALS__.invoke(cmd, args);
  const say = (ok, msg) => invoke('qa_log', { line: (ok ? 'PASS ' : 'FAIL ') + msg });
  const $ = s => document.querySelector(s), $$ = s => [...document.querySelectorAll(s)];
  const ok = async (check, message) => { let pass = false; for (let i = 0; i < 40 && !(pass = !!(await check())); i++) await wait(100); say(pass, message()); };
  const dialogTitle = () => $('dialog[open] #card-title')?.textContent.trim();
  const close = async () => { $('dialog[open]')?.close(); await wait(300); };
  try {
    await wait(1500);
    const qa = window.__finanzas; qa.capture = true; // right-click menus are recorded instead of shown
    const card = (await invoke('accounts_overview')).find(a => a.name === 'Tarjeta');
    await invoke('set_card_terms', { accountId: card.id, terms: { creditLimit: 1000000, cutDay: 12, dueDay: 2, annualRate: 4550, minPayment: 500 } });
    await wait(1500); // the app polls the data version

    location.hash = `cuenta-${card.id}`; await wait(1200);
    const pageBtn = () => $$('.topbar .btn').find(b => b.textContent.includes('Datos de la tarjeta'));
    await ok(pageBtn, () => 'the card page has a "Datos de la tarjeta" button');
    pageBtn()?.click();
    await ok(() => dialogTitle() === 'Tarjeta', () => `it opens the card data dialog (${dialogTitle()})`);
    await close();

    const main = $$('.sidebar .nav').find(b => b.textContent.includes('Cuenta principal'));
    if (main) { qa.lastMenu = null; main.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true })); }
    await ok(() => qa.lastMenu && !qa.lastMenu.some(x => x && x[0] === 'Datos de la tarjeta…'), () => 'a plain account has no card data item');
    const side = $$('.sidebar .nav').find(b => b.textContent.includes('Tarjeta'));
    qa.lastMenu = null; side.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
    await ok(() => qa.lastMenu?.some(x => x && x[0] === 'Datos de la tarjeta…'), () => 'the card has "Datos de la tarjeta…" on right-click');
    qa.lastMenu.find(x => x && x[0] === 'Datos de la tarjeta…')[1]();
    await ok(() => dialogTitle() === 'Tarjeta', () => 'the menu item opens the dialog');
    await close();

    // Deudas is off by default in the demo data: switch it on first.
    location.hash = 'ajustes/deudas'; await wait(800);
    const sw = [...document.querySelectorAll('label.setting')].find(l => l.textContent.includes('Mostrar la página Deudas'))?.querySelector('input');
    if (sw && !sw.checked) { sw.click(); await wait(500); }
    location.hash = 'deudas'; await wait(1200);
    const line = () => $('.terms .link');
    await ok(() => line()?.textContent.trim() === 'Tasa 45.5% · Corte día 12 · Pago día 2 · Mínimo 5% · Límite $10,000.00', () => `Deudas shows the card data (${line()?.textContent.trim()})`);
    line()?.click();
    await ok(() => dialogTitle() === 'Tarjeta', () => 'clicking the data line opens the dialog');
    await close();
    const datos = $$('.debt-h .btn').find(b => b.textContent.trim() === 'Datos');
    await ok(() => datos, () => 'the pencil is now a "Datos" button');
    datos?.click();
    await ok(() => dialogTitle() === 'Tarjeta', () => '"Datos" opens the dialog');
    await close();
  } catch (e) { say(false, 'exception: ' + e); }
  invoke('qa_log', { line: 'DONE' });
})();
