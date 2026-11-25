// Visual review: walks every screen in light and dark; qa/shots.sh captures the window on each SHOT line.
(async () => {
  const wait = ms => window.__TAURI_INTERNALS__.invoke('qa_sleep', { ms });
  const log = line => window.__TAURI_INTERNALS__.invoke('qa_log', { line });
  const screens = ['resumen', 'movimientos', 'presupuesto', 'proximos', 'deudas', 'deseos', 'ajustes'];
  try {
    await wait(1500);
    // Deudas is off by default; switch it on so its screen is captured too.
    location.hash = 'ajustes/deudas'; await wait(800);
    const sw = [...document.querySelectorAll('label.setting')].find(l => l.textContent.includes('Mostrar la página Deudas'))?.querySelector('input');
    if (sw && !sw.checked) { sw.click(); await wait(500); }
    for (const theme of ['light', 'dark']) {
      document.documentElement.dataset.theme = theme;
      for (const screen of screens) {
        location.hash = screen;
        await wait(1200);
        scrollTo(0, 0);
        await log(`SHOT ${screen}-${theme}`);
        await wait(1500); // the shell captures meanwhile
      }
    }
  } catch (e) { await log('FAIL shots: ' + e); }
  await log('DONE');
})();
