// Bundled fonts: the UI and the amounts render with the app's own faces, not the system's.
(async () => {
  const wait = ms => window.__TAURI_INTERNALS__.invoke('qa_sleep', { ms });
  const say = (ok, msg) => window.__TAURI_INTERNALS__.invoke('qa_log', { line: (ok ? 'PASS ' : 'FAIL ') + msg });
  try {
    await wait(1500);
    await document.fonts.ready;
    const loaded = [...document.fonts].filter(f => f.status === 'loaded').map(f => `${f.family} ${f.weight}`);
    say(document.fonts.check('600 15px "Funnel Sans"'), `Funnel Sans is loaded (${loaded.join(', ')})`);
    say(document.fonts.check('500 15px "DM Mono"'), 'DM Mono is loaded');
    const body = getComputedStyle(document.body).fontFamily;
    say(body.replace(/"/g, "").startsWith("Funnel Sans"), `the UI uses Funnel Sans (${body})`);
    const money = document.querySelector('.money');
    say(money && getComputedStyle(money).fontFamily.replace(/"/g, "").startsWith("DM Mono"), 'amounts use DM Mono');
  } catch (e) { say(false, 'script error: ' + e); }
  window.__TAURI_INTERNALS__.invoke('qa_log', { line: 'DONE' });
})();
