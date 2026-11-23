// Ajustes: sub-tabs, profiles (create/switch/rename/hide/delete with confirmation), themes and reminder settings persisted.
// Does not toggle "Abrir al iniciar sesión" nor send a test notification: both would touch the real Mac.
(async () => {
  const wait = ms => window.__TAURI_INTERNALS__.invoke('qa_sleep', { ms }); // webview timers stop when the window is covered
  const inv = (c, a) => window.__TAURI_INTERNALS__.invoke(c, a);
  const log = document.createElement('pre');
  log.style.cssText = 'position:fixed;left:270px;bottom:10px;z-index:99999;background:#000c;color:#7f7;font:12px monospace;padding:8px;max-width:680px;white-space:pre-wrap;border-radius:8px';
  document.body.append(log);
  const say = (ok, msg) => { const line = (ok ? 'PASS ' : 'FAIL ') + msg; log.textContent += line + '\n'; inv('qa_log', { line }); };
  const $ = s => document.querySelector(s);
  // Waits until check() is truthy (or ~5 s), so slow refreshes do not make the script flaky.
  const until = async check => { for (let i = 0; i < 50 && !(await check()); i++) await wait(100); return check(); };
  const set = (el, v, ev = 'input') => { el.value = v; el.dispatchEvent(new Event(ev, { bubbles: true })); };
  const btn = (root, text) => [...root.querySelectorAll('button')].find(b => b.textContent.trim().includes(text));
  const profileRow = label => [...document.querySelectorAll('.card .row')].find(r => r.querySelector('.title')?.textContent === label);
  const avatar = () => $('.side-foot .avatar')?.textContent.trim();
  try {
    await wait(1200); location.hash = 'ajustes'; await wait(1000);
    say(document.querySelectorAll('.subnav .nav').length === 9 && $('.topbar .sub')?.textContent === 'Perfil', 'nine sub-tabs, Perfil first');
    say(!!profileRow('Personal')?.textContent.includes('En uso'), 'Personal is the profile in use');

    set($('input[aria-label="Nombre del nuevo perfil"]'), 'QA Negocio');
    $('input[aria-label="Nombre del nuevo perfil"]').form.requestSubmit(); await until(() => avatar() === 'Q' && profileRow('Qa negocio'));
    say(profileRow('Qa negocio')?.textContent.includes('En uso') && avatar() === 'Q', `new profile is created and active (avatar "${avatar()}")`);
    await until(() => avatar() === 'Q');
    location.hash = 'movimientos'; await until(() => $('table'));
    say(document.querySelectorAll('tr.tx').length === 0, 'the new profile starts with no movements');
    location.hash = 'ajustes/perfil'; await wait(800);
    btn(profileRow('Personal'), 'Cambiar').click(); await until(() => avatar() === 'P');
    location.hash = 'movimientos'; await until(() => document.querySelectorAll('tr.tx').length > 0);
    say(document.querySelectorAll('tr.tx').length > 0 && avatar() === 'P', `switching back to Personal shows its ${document.querySelectorAll('tr.tx').length} movements`);
    location.hash = 'ajustes/perfil'; await wait(800);

    btn(profileRow('Qa negocio'), 'Cambiar').click(); await until(() => avatar() === 'Q');
    set($('.card input.input'), 'QA Tienda', 'change'); await until(() => profileRow('Qa tienda'));
    say(!!profileRow('Qa tienda') && !profileRow('Qa negocio') && avatar() === 'Q', 'renaming the active profile keeps it open');
    btn(profileRow('Personal'), 'Cambiar').click(); await until(() => avatar() === 'P');
    btn(profileRow('Qa tienda'), 'Ocultar').click(); await until(() => profileRow('Qa tienda')?.textContent.includes('Oculto'));
    say(profileRow('Qa tienda')?.textContent.includes('Oculto') && (await inv('get_settings')).hidden_profiles.includes('qa-tienda'), 'Ocultar marks it hidden and saves it');
    btn(profileRow('Qa tienda'), 'Mostrar').click(); await until(() => !profileRow('Qa tienda')?.textContent.includes('Oculto'));

    profileRow('Qa tienda').querySelector('button[aria-label^="Eliminar"]').click(); await wait(400);
    const confirm = $('dialog[open]');
    say(!!confirm && confirm.textContent.includes('no se puede deshacer') && document.activeElement?.textContent.trim() === 'Conservar', 'deleting a profile asks first, focus on "Conservar"');
    btn(confirm, 'Conservar').click(); await wait(800);
    say(!!profileRow('Qa tienda'), '"Conservar" keeps it');
    profileRow('Qa tienda').querySelector('button[aria-label^="Eliminar"]').click(); await wait(400);
    btn($('dialog[open]'), 'Eliminar para siempre').click(); await until(() => !profileRow('Qa tienda'));
    say(!profileRow('Qa tienda') && (await inv('profiles')).list.length === 1, 'confirming deletes it');

    location.hash = 'ajustes/general'; await wait(800);
    [...document.querySelectorAll('.theme-card')].find(c => c.textContent.includes('Arena')).querySelector('input').click(); await until(() => document.documentElement.dataset.palette === 'arena');
    say(document.documentElement.dataset.palette === 'arena' && (await inv('get_settings')).palette === 'arena', 'choosing Arena applies and saves the theme');
    btn($('.seg[aria-label="Modo claro u oscuro"]'), 'Oscuro').click(); await wait(800);
    say(document.documentElement.dataset.theme === 'dark' && (await inv('get_settings')).dark === true, 'Oscuro forces dark mode');
    btn($('.seg[aria-label="Modo claro u oscuro"]'), 'Sistema').click(); await wait(800);
    say(!document.documentElement.dataset.theme && (await inv('get_settings')).dark === null, 'Sistema follows the system again');

    location.hash = 'ajustes/avisos'; await wait(800);
    btn($('.stepper'), '+').click(); await wait(800);
    say((await inv('get_settings')).days === 3 && $('.stepper output').textContent.includes('3 días'), 'days stepper saves 3 días');
    $('input[role=switch]').click(); await wait(800);
    say((await inv('get_settings')).enabled === false && $('.stepper button').disabled, 'turning reminders off saves it and disables the steppers');
    location.hash = 'ajustes/atajos'; await wait(600);
    say($('.keys')?.textContent.includes('⌘Z'), 'Atajos lists the shortcuts');
  } catch (e) { say(false, 'exception: ' + e); }
  inv('qa_log', { line: 'DONE' });
})();
