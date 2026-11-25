// Collapsible sidebar: the toggle and the command shrink it to icons, labels stay accessible, the choice is remembered.
(async () => {
  const wait = ms => window.__TAURI_INTERNALS__.invoke('qa_sleep', { ms });
  const say = (ok, msg) => window.__TAURI_INTERNALS__.invoke('qa_log', { line: (ok ? 'PASS ' : 'FAIL ') + msg });
  const $ = s => document.querySelector(s);
  const ok = async (check, message) => { let pass = false; for (let i = 0; i < 40 && !(pass = !!(await check())); i++) await wait(100); say(pass, message()); };
  const width = () => Math.round($('.sidebar').getBoundingClientRect().width);
  try {
    await wait(1500);
    if ($('.sidebar.collapsed')) { window.__finanzas.runCommand('sidebar'); await wait(600); } // a previous run may have left it collapsed
    const open = width();
    $('.side-toggle').click();
    await wait(500); // let the width transition finish
    await ok(() => $('.sidebar.collapsed') && width() < 70, () => `the toggle collapses it (${open}px → ${width()}px)`);
    await ok(() => localStorage.getItem('sidebar-collapsed') === '1', () => 'the choice is saved');
    const nav = [...document.querySelectorAll('.sidebar .nav')].find(b => b.title === 'Movimientos');
    await ok(() => nav && nav.textContent.includes('Movimientos') && nav.getBoundingClientRect().width < 70, () => 'icons keep their name as tooltip and accessible text');
    await ok(() => getComputedStyle($('.sidebar .section')).opacity === '0' && getComputedStyle($('.sidebar .brand-text')).opacity === '0', () => 'section titles and the brand fade out');
    const icon = el => Math.round(el.querySelector('svg').getBoundingClientRect().left);
    const side = $('.sidebar').getBoundingClientRect();
    await ok(() => Math.abs(icon(nav) + 9 - (side.left + side.right) / 2) <= 2, () => `icons end centred in the bar (icon at ${icon(nav)}px)`);
    nav.click();
    await ok(() => location.hash === '#movimientos', () => 'collapsed icons still navigate');
    window.__finanzas.runCommand('sidebar'); await wait(500);
    await ok(() => !$('.sidebar.collapsed') && width() === open, () => `the command expands it again (${width()}px)`);
    await ok(() => localStorage.getItem('sidebar-collapsed') === '0', () => 'the expanded choice is saved');
    location.hash = 'resumen'; await wait(800);
  } catch (e) { say(false, 'exception: ' + e); }
  window.__TAURI_INTERNALS__.invoke('qa_log', { line: 'DONE' });
})();
