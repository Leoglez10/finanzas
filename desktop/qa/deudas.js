// Deudas: off until enabled in Ajustes; a card with optional terms, the notice for missing ones, advice and installments; Resumen widget.
(async () => {
  const wait = ms => window.__TAURI_INTERNALS__.invoke('qa_sleep', { ms });
  const inv = (c, a) => window.__TAURI_INTERNALS__.invoke(c, a);
  const log = document.createElement('pre');
  log.style.cssText = 'position:fixed;left:270px;bottom:10px;z-index:99999;background:#000c;color:#7f7;font:12px monospace;padding:8px;max-width:680px;white-space:pre-wrap;border-radius:8px';
  document.body.append(log);
  const say = (ok, msg) => { const line = (ok ? 'PASS ' : 'FAIL ') + msg; log.textContent += line + '\n'; inv('qa_log', { line }); };
  const $ = s => document.querySelector(s);
  const ok = async (check, message) => { let pass = false; for (let i = 0; i < 60 && !(pass = !!(await check())); i++) await wait(100); say(pass, message()); };
  const set = (el, v) => { el.value = v; el.dispatchEvent(new Event('input', { bubbles: true })); };
  const nav = name => [...document.querySelectorAll('.sidebar .nav')].find(b => b.textContent.includes(name));
  const field = (d, label) => [...d.querySelectorAll('label.field')].find(l => l.textContent.trim().startsWith(label))?.querySelector('input,select');
  const main = () => $('#main').textContent;
  try {
    localStorage.removeItem('finanzas.offer.deudas');
    location.hash = 'ajustes'; await wait(300); location.hash = 'resumen';
    await wait(1200);
    say(!nav('Deudas'), 'Deudas is not in the sidebar until enabled');
    const offer = () => $('#main .offer');
    await ok(() => offer()?.textContent.includes('Nuevo: página Deudas'), () => 'an account in debt makes the Resumen offer the Deudas page');
    [...offer().querySelectorAll('button')].find(b => b.textContent === 'Ahora no').click();
    await ok(() => !offer(), () => '"Ahora no" hides the offer');
    location.hash = 'movimientos'; await wait(300); location.hash = 'resumen'; await wait(800);
    say(!offer(), 'and it stays hidden');
    location.hash = 'ajustes/deudas';
    await ok(() => $('input[role=switch]') && main().includes('Mostrar la página Deudas'), () => 'Ajustes has a Deudas section');
    const sw = [...document.querySelectorAll('label.setting')].find(l => l.textContent.includes('Mostrar la página Deudas')).querySelector('input');
    sw.click();
    await ok(() => nav('Deudas'), () => 'turning it on adds Deudas to the sidebar');

    nav('Nueva cuenta').click();
    await ok(() => $('dialog[open] input[placeholder="Cuenta principal"]'), () => 'new account dialog opens');
    let d = $('dialog[open]');
    set(d.querySelector('input[placeholder="Cuenta principal"]'), 'QA Visa');
    set(d.querySelector('input[placeholder="0.00"]'), '-3,000');
    [...d.querySelectorAll('label.setting')].find(l => l.textContent.includes('Tarjeta de crédito')).querySelector('input').click();
    await ok(() => field(d, 'Límite de crédito'), () => 'the credit-card switch shows the optional card fields');
    await ok(() => d.textContent.includes('Todo es opcional'), () => 'the notice explains what the data unlocks');
    set(field(d, 'Día de corte'), '40');
    d.querySelector('button[value="save"]').click(); await wait(200);
    await ok(() => d.querySelector('.err')?.textContent.includes('1 al 31'), () => `an invalid day is refused: "${d.querySelector('.err')?.textContent}"`);
    set(field(d, 'Día de corte'), '');
    set(field(d, 'Límite de crédito'), '10,000');
    d.querySelector('button[value="save"]').click();
    await ok(() => nav('QA Visa'), () => 'the card is created with only the limit');

    nav('Deudas').click();
    await ok(() => main().includes('QA Visa') && main().includes('$3,000.00'), () => 'Deudas lists the card with its debt');
    await ok(() => main().includes('Disponible $7,000.00 de $10,000.00'), () => 'available credit from the limit');
    await ok(() => main().includes('Agrega día de corte, día de pago, tasa y pago mínimo'), () => 'missing terms are named with what they unlock');
    await ok(() => main().includes('Usas el 30%') === false && main().includes('usas 30%'), () => 'utilization shown');

    [...document.querySelectorAll('#main button')].find(b => b.textContent.includes('Completar datos')).click();
    await ok(() => $('dialog[open] #card-title'), () => 'Completar datos opens the card dialog');
    d = $('dialog[open]');
    const today = new Date();
    set(field(d, 'Día de corte'), '1'); set(field(d, 'Día límite de pago'), '28'); set(field(d, 'Tasa anual'), '60'); set(field(d, 'Pago mínimo'), '10');
    d.querySelector('button[value="save"]').click();
    await ok(() => main().includes('Saldo al corte') && main().includes('Para no generar intereses'), () => `full terms show the statement facts (today ${today.toDateString()})`);
    await ok(() => !main().includes('Completar datos'), () => 'the missing-data notice goes away');

    [...document.querySelectorAll('#main .plans button')].find(b => b.textContent.includes('Agregar')).click();
    await ok(() => $('dialog[open] #plan-title'), () => 'installment dialog opens');
    d = $('dialog[open]');
    set(field(d, 'Qué compraste'), 'Laptop'); set(field(d, 'Monto total'), '1,200'); set(field(d, 'Meses'), '12');
    await ok(() => d.textContent.includes('$100.00 al mes'), () => 'the dialog previews the monthly charge');
    d.querySelector('button[value="save"]').click();
    await ok(() => main().includes('Laptop') && main().includes('$100.00 al mes'), () => 'the installment appears under the card');
    await ok(() => main().includes('Mensualidades a meses'), () => 'KPIs show the monthly installments');

    location.hash = 'resumen';
    await ok(() => [...document.querySelectorAll('#main h2')].some(h => h.textContent === 'Deudas') && main().includes('QA Visa'), () => 'the Resumen has the Deudas widget');
    location.hash = 'ajustes/deudas'; await wait(300);
    [...document.querySelectorAll('label.setting')].find(l => l.textContent.includes('Mostrar la página Deudas')).querySelector('input').click();
    await ok(() => !nav('Deudas'), () => 'turning it off removes Deudas from the sidebar');
    location.hash = 'resumen';
    await ok(() => main().includes('Saldo por cuenta') && ![...document.querySelectorAll('#main h2')].some(h => h.textContent === 'Deudas'), () => 'and hides the widget');
    localStorage.removeItem('finanzas.offer.deudas');
    location.hash = 'movimientos'; await wait(300); location.hash = 'resumen';
    await ok(() => offer(), () => 'the offer comes back while the page is off and was not dismissed');
    [...offer().querySelectorAll('button')].find(b => b.textContent === 'Activar').click();
    await ok(() => location.hash === '#deudas' && nav('Deudas') && main().includes('Debes en total'), () => '"Activar" turns the page on and opens it');
  } catch (e) { say(false, 'exception: ' + e); }
  inv('qa_log', { line: 'DONE' });
})();
