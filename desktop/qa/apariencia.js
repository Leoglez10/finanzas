// Account look: icon and color when creating, and later from the account page ("Apariencia"), shown in the sidebar.
(async () => {
  const wait = ms => window.__TAURI_INTERNALS__.invoke('qa_sleep', { ms });
  const inv = (c, a) => window.__TAURI_INTERNALS__.invoke(c, a);
  const say = (ok, msg) => inv('qa_log', { line: (ok ? 'PASS ' : 'FAIL ') + msg });
  const $ = s => document.querySelector(s);
  const ok = async (check, message) => { let pass = false; for (let i = 0; i < 60 && !(pass = !!(await check())); i++) await wait(100); say(pass, message()); };
  const set = (el, v) => { el.value = v; el.dispatchEvent(new Event('input', { bubbles: true })); };
  const nav = name => [...document.querySelectorAll('.sidebar .nav')].find(b => b.textContent.includes(name));
  const badgeIcon = el => el?.querySelector('.badge use')?.getAttribute('href');
  try {
    await wait(1200);
    nav('Nueva cuenta').click();
    await ok(() => $('dialog[open] [aria-label="Icono de la cuenta"]'), () => 'the new account dialog has the icon picker');
    let d = $('dialog[open]');
    set(d.querySelector('input[placeholder="Cuenta principal"]'), 'QA Banco');
    d.querySelector('[aria-label="Icono de la cuenta"] [aria-label="Banco"]').click();
    d.querySelector('[aria-label="Color de la cuenta"] [aria-label="Azul"]').click();
    d.querySelector('button[value="save"]').click();
    await ok(() => badgeIcon(nav('QA Banco')) === '#i-bank' && nav('QA Banco').querySelector('.badge.tinted'), () => `the sidebar shows the bank icon tinted (${badgeIcon(nav('QA Banco'))})`);
    const acc = (await inv('accounts_overview')).find(a => a.name === 'QA Banco');
    say(acc.icon === 'bank' && acc.color === 'blue' && acc.image === null, 'icon and color are saved: ' + JSON.stringify([acc.icon, acc.color]));
    say(badgeIcon(nav('Cuenta principal')) === '#i-card', 'accounts without a choice keep the card icon');

    await ok(() => location.hash === '#cuenta-' + acc.id, () => 'it opens the new account');
    [...document.querySelectorAll('#main button')].find(b => b.textContent.includes('Apariencia')).click();
    await ok(() => $('dialog[open] #style-title'), () => 'Apariencia opens the style dialog');
    d = $('dialog[open]');
    say(d.querySelector('[aria-label="Banco"]').getAttribute('aria-checked') === 'true', 'it starts with the saved icon');
    d.querySelector('[aria-label="Ahorro"]').click();
    d.querySelector('[aria-label="Verde"]').click();
    await ok(() => badgeIcon(d.querySelector('.preview')) === '#i-piggy', () => 'the preview follows the choice');
    say(d.textContent.includes('Elegir imagen'), 'it offers an own image');
    d.querySelector('button[value="save"]').click();
    await ok(() => badgeIcon(nav('QA Banco')) === '#i-piggy', () => 'Guardar updates the sidebar');
    await ok(() => $('.toast')?.textContent.includes('Deshacer'), () => 'with undo');
    [...document.querySelectorAll('.toast button')].find(b => b.textContent.includes('Deshacer'))?.click();
    await ok(() => badgeIcon(nav('QA Banco')) === '#i-bank', () => 'undo restores the previous icon');
    const bad = await inv('set_account_style', { accountId: acc.id, icon: 'rocket', color: null }).catch(e => e);
    say(bad === 'Icono desconocido', 'unknown icons are refused: ' + bad);
  } catch (e) { say(false, 'exception: ' + e); }
  inv('qa_log', { line: 'DONE' });
})();
