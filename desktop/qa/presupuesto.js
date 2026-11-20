// Presupuesto: assign with +N, undo, invalid text, copy from previous month.
(async () => {
  const wait = ms => window.__TAURI_INTERNALS__.invoke('qa_sleep', { ms }); // webview timers stop when the window is covered
  const log = document.createElement('pre');
  log.style.cssText = 'position:fixed;left:270px;bottom:10px;z-index:99999;background:#000c;color:#7f7;font:12px monospace;padding:8px;max-width:640px;white-space:pre-wrap;border-radius:8px';
  document.body.append(log);
  const say = (ok, msg) => { const line = (ok ? 'PASS ' : 'FAIL ') + msg; log.textContent += line + '\n'; window.__TAURI_INTERNALS__.invoke('qa_log', { line }); };
  const $ = s => document.querySelector(s);
  // Each check retries for up to ~6 s: macOS slows a covered or sleeping app, so fixed waits are not enough.
  const ok = async (check, message) => { let pass = false; for (let i = 0; i < 60 && !(pass = !!(await check())); i++) await wait(100); say(pass, message()); };
  const input = name => $(`input[aria-label="Asignado a ${name}"]`);
  const change = (el, v) => { el.value = v; el.dispatchEvent(new Event('change', { bubbles: true })); };
  try {
    await wait(1200); location.hash = 'presupuesto'; await wait(1000);
    await ok(() => document.querySelectorAll('.budget-row:not(.head)').length >= 7, () => `rows: ${document.querySelectorAll('.budget-row:not(.head)').length}`);
    await ok(() => !!$('.chip.bad') && !!$('.chip.warn'), () => 'states include Excedido and Casi al límite');
    const hero = $('.hero .money').textContent;
    change(input('Vivienda'), '+100'); await wait(1200);
    await ok(() => input('Vivienda').value === '900.00', () => `+100 on 800 gives ${input('Vivienda').value}`);
    await ok(() => $('.hero .money').textContent !== hero, () => `available to assign updates: ${hero} → ${$('.hero .money').textContent}`);
    await ok(() => $('.toast.show')?.textContent.includes('Deshacer'), () => `toast: "${$('.toast.show')?.textContent.trim()}"`);
    $('.toast.show button').click(); await wait(1200);
    await ok(() => input('Vivienda').value === '800.00', () => `undo restores ${input('Vivienda').value}`);
    change(input('Ocio'), 'abc'); await wait(800);
    await ok(() => $('.toast.show')?.textContent.includes('monto válido') && input('Ocio').value === '300.00', () => `invalid text is rejected and reset (${input('Ocio').value})`);
    change(input('Ocio'), '150*2+50'); await wait(1000);
    await ok(() => input('Ocio').value === '350.00', () => `arithmetic 150*2+50 gives ${input('Ocio').value}`);
    $('.month button[aria-label="Mes siguiente"]').click(); await wait(900);
    const copy = [...document.querySelectorAll('.hero button')].find(b => b.textContent.includes('Copiar'));
    copy.click(); await wait(2500);
    await ok(() => input('Ocio')?.value === '350.00' && input('Vivienda')?.value === '800.00', () => `copy from previous month: Ocio ${input('Ocio')?.value}, Vivienda ${input('Vivienda')?.value}`);
    $('.month button[aria-label="Mes anterior"]').click(); await wait(800);
  } catch (e) { say(false, 'exception: ' + e); }
  window.__TAURI_INTERNALS__.invoke('qa_log', { line: 'DONE' });
})();
