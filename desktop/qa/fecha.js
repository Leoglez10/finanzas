// Date fields use the app's own calendar popover, on top of the dialog that holds them.
(async () => {
  const wait = ms => window.__TAURI_INTERNALS__.invoke('qa_sleep', { ms });
  const say = (ok, msg) => window.__TAURI_INTERNALS__.invoke('qa_log', { line: (ok ? 'PASS ' : 'FAIL ') + msg });
  const $ = s => document.querySelector(s), $$ = s => [...document.querySelectorAll(s)];
  const ok = async (check, message) => { let pass = false; for (let i = 0; i < 40 && !(pass = !!(await check())); i++) await wait(100); say(pass, message()); };
  try {
    await wait(1200);
    location.hash = 'proximos'; await wait(1200);
    $$('.card .row').find(r => r.querySelector('.date') && r.querySelector('.title')?.textContent.trim() === 'Renta').querySelector('.title button').click();
    await ok(() => $('dialog[open] .date-btn'), () => 'the schedule dialog shows the date button, no native input');
    await ok(() => !$('dialog[open] input[type="date"]'), () => 'no native date input left');
    $('dialog[open] .date-btn').click();
    await ok(() => $('dialog[open] .date-pop:popover-open'), () => 'the button opens the calendar');
    const top = () => { const r = $('dialog[open] .date-pop').getBoundingClientRect(); return document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2)?.closest('dialog[open] .date-pop'); };
    await ok(top, () => 'the calendar sits on top of the dialog');
    await ok(() => $('dialog[open] .date-pop .d[aria-pressed="true"]'), () => 'the current date is selected');
    const pick = $$('dialog[open] .date-pop .d:not(.out)').find(b => b.textContent === '20');
    pick.click();
    await ok(() => !$('dialog[open] .date-pop:popover-open') && /20/.test($('dialog[open] .date-btn').textContent), () => `picking a day closes it and shows it (${$('dialog[open] .date-btn').textContent.trim()})`);
    await ok(() => $('dialog[open]'), () => 'the dialog stays open');
    $('dialog[open] .date-btn').click(); await wait(400);
  } catch (e) { say(false, 'exception: ' + e); }
  window.__TAURI_INTERNALS__.invoke('qa_log', { line: 'DONE' });
})();
