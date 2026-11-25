// Parity with the Rune app: MCP agents tab, database location, "Primeros pasos" on an empty profile.
(async () => {
  const wait = ms => window.__TAURI_INTERNALS__.invoke('qa_sleep', { ms }); // webview timers stop when the window is covered
  const inv = (c, a) => window.__TAURI_INTERNALS__.invoke(c, a);
  const log = document.createElement('pre');
  log.style.cssText = 'position:fixed;left:270px;bottom:10px;z-index:99999;background:#000c;color:#7f7;font:12px monospace;padding:8px;max-width:680px;white-space:pre-wrap;border-radius:8px';
  document.body.append(log);
  const say = (ok, msg) => { const line = (ok ? 'PASS ' : 'FAIL ') + msg; log.textContent += line + '\n'; inv('qa_log', { line }); };
  const $ = s => document.querySelector(s);
  const ok = async (check, message) => { let pass = false; for (let i = 0; i < 60 && !(pass = !!(await check())); i++) await wait(100); say(pass, message()); };
  try {
    await wait(1200);
    location.hash = 'ajustes/agentes';
    await ok(() => document.querySelectorAll('.card pre').length === 3, () => `3 agents not connected, each with a config (${document.querySelectorAll('.card pre').length})`);
    await ok(() => [...document.querySelectorAll('.card pre')].every(p => p.textContent.includes('finanzas-mcp')), () => 'configs point at finanzas-mcp next to the app');
    await ok(() => $('.card').textContent.includes('Claude') && $('.card').textContent.includes('Codex') && $('.card').textContent.includes('OpenCode'), () => 'Claude, Codex and OpenCode are listed');
    location.hash = 'ajustes/datos';
    await ok(() => [...document.querySelectorAll('.card pre')].some(p => p.textContent.trim().endsWith('finanzas.db')), () => `database location shown: ${[...document.querySelectorAll('.card pre')].map(p => p.textContent.trim()).join(' ')}`);

    location.hash = 'resumen';
    await ok(() => $('.page .card-h h2')?.textContent && ![...document.querySelectorAll('.card-h h2')].some(h => h.textContent === 'Primeros pasos'), () => 'a database with data shows no "Primeros pasos"');
    await inv('create_profile', { name: 'QA Vacio' }); await inv('qa_sleep', { ms: 200 });
    location.hash = 'resumen';
    await ok(() => [...document.querySelectorAll('.card-h h2')].some(h => h.textContent === 'Primeros pasos') && document.querySelectorAll('.card .row .btn.sm').length >= 3, () => 'an empty profile shows "Primeros pasos" with 3 actions');
    [...document.querySelectorAll('.card .row .btn.sm')].find(b => b.textContent.includes('Nueva cuenta')).click();
    await ok(() => $('dialog[open] input[placeholder="Cuenta principal"]'), () => '"Nueva cuenta" from Primeros pasos opens the dialog');
    $('dialog[open]').close();
    await inv('switch_profile', { slug: 'personal' });
  } catch (e) { say(false, 'exception: ' + e); }
  inv('qa_log', { line: 'DONE' });
})();
