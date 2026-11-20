// New account with an opening balance; validation; the opening becomes a "Saldo inicial" movement.
(async () => {
  const wait = ms => window.__TAURI_INTERNALS__.invoke('qa_sleep', { ms }); // webview timers stop when the window is covered
  const inv = (c, a) => window.__TAURI_INTERNALS__.invoke(c, a);
  const log = document.createElement('pre');
  log.style.cssText = 'position:fixed;left:270px;bottom:10px;z-index:99999;background:#000c;color:#7f7;font:12px monospace;padding:8px;max-width:680px;white-space:pre-wrap;border-radius:8px';
  document.body.append(log);
  const say = (ok, msg) => { const line = (ok ? 'PASS ' : 'FAIL ') + msg; log.textContent += line + '\n'; inv('qa_log', { line }); };
  const $ = s => document.querySelector(s);
  const ok = async (check, message) => { let pass = false; for (let i = 0; i < 60 && !(pass = !!(await check())); i++) await wait(100); say(pass, message()); };
  const set = (el, v) => { el.value = v; el.dispatchEvent(new Event('input', { bubbles: true })); };
  const nav = name => [...document.querySelectorAll('.sidebar .nav')].find(b => b.textContent.includes(name));
  try {
    await wait(1200);
    nav('Nueva cuenta').click();
    await ok(() => $('dialog[open] input[placeholder="Cuenta principal"]'), () => '"Nueva cuenta" in the sidebar opens the dialog');
    const d = $('dialog[open]');
    d.querySelector('button[value="save"]').click(); await wait(200);
    await ok(() => d.querySelector('.err')?.textContent.includes('Ponle un nombre'), () => `empty name is rejected: "${d.querySelector('.err')?.textContent}"`);
    set(d.querySelector('input[placeholder="Cuenta principal"]'), 'QA Ahorros');
    set(d.querySelector('input[placeholder="0.00"]'), '1,2');
    d.querySelector('button[value="save"]').click(); await wait(200);
    await ok(() => d.querySelector('.err')?.textContent.includes('saldo válido'), () => 'invalid opening balance is rejected');
    set(d.querySelector('input[placeholder="0.00"]'), '1,500');
    d.querySelector('button[value="save"]').click();
    await ok(() => nav('QA Ahorros')?.textContent.includes('$1,500.00'), () => `the account appears with $1,500.00 (${nav('QA Ahorros')?.textContent.trim()})`);
    await ok(() => location.hash.startsWith('#cuenta-') && [...document.querySelectorAll('tr.tx')].some(r => r.textContent.includes('Saldo inicial') && r.textContent.includes('+$1,500.00')), () => 'it opens the account with a "Saldo inicial" movement');
  } catch (e) { say(false, 'exception: ' + e); }
  inv('qa_log', { line: 'DONE' });
})();
