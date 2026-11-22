// Frame timing for the Resumen editor: drag a widget across the grid and resize one, one pointer move per frame.
// Reports median / p95 / max frame time and frames over 20 ms (60 Hz budget is 16.7 ms).
(async () => {
  const wait = ms => window.__TAURI_INTERNALS__.invoke('qa_sleep', { ms });
  const inv = (c, a) => window.__TAURI_INTERNALS__.invoke(c, a);
  const say = (ok, msg) => inv('qa_log', { line: (ok ? 'PASS ' : 'FAIL ') + msg });
  const $ = s => document.querySelector(s);
  const frame = () => new Promise(r => requestAnimationFrame(r));
  const fire = (type, el, x, y) => el.dispatchEvent(new PointerEvent(type, { bubbles: true, cancelable: true, clientX: x, clientY: y, pointerId: 1, isPrimary: true }));
  // Runs `step(i)` once per frame for `n` frames and returns the frame intervals.
  async function frames(n, step) {
    const times = [];
    let last = await frame();
    for (let i = 0; i < n; i++) { step(i); const t = await frame(); times.push(t - last); last = t; }
    return times;
  }
  function stats(name, times) {
    const s = [...times].sort((a, b) => a - b);
    const q = p => s[Math.min(s.length - 1, Math.floor(p * s.length))];
    const slow = times.filter(t => t > 20).length;
    say(true, `${name}: ${times.length} cuadros · mediana ${q(.5).toFixed(1)} ms · p95 ${q(.95).toFixed(1)} ms · máx ${s.at(-1).toFixed(1)} ms · lentos (>20 ms) ${slow}`);
  }
  try {
    location.hash = 'resumen';
    await wait(1500);
    const idle = await frames(60, () => {});
    stats('reposo', idle);
    [...document.querySelectorAll('.topbar button, header button')].find(b => b.textContent.includes('Personalizar'))?.click();
    await wait(400);
    if (!$('.dash.editing')) { say(false, 'no entró al modo edición'); return inv('qa_log', { line: 'DONE' }); }

    // Drag: take the first widget's handle and sweep down and back across the grid.
    const handle = $('.dash .w .handle');
    const grid = $('.dash').getBoundingClientRect();
    const h0 = handle.getBoundingClientRect();
    fire('pointerdown', handle, h0.left + 5, h0.top + 5);
    await wait(100);
    const n = 120, path = i => { const t = i / (n - 1), k = t < .5 ? t * 2 : 2 - t * 2; return [grid.left + 40 + k * (grid.width - 80), grid.top + 40 + k * (Math.min(grid.height, innerHeight - grid.top) - 80)]; };
    const drag = await frames(n, i => { const [x, y] = path(i); fire('pointermove', window, x, y); });
    fire('pointerup', window, ...path(n - 1));
    stats('arrastrar', drag);
    await wait(500);

    // Resize: grab a half-width widget's corner and sweep its width between 2 and 6 columns, twice.
    const grip = [...document.querySelectorAll('.dash .w:not(.off)')].find(w => w.style.gridColumn.includes('3'))?.querySelector('.grip');
    if (!grip) { say(false, 'no hay widget de 3 columnas para cambiar de tamaño'); return inv('qa_log', { line: 'DONE' }); }
    const w = grip.closest('.w').getBoundingClientRect(), g0 = grip.getBoundingClientRect();
    fire('pointerdown', grip, g0.left + 4, g0.top + 4);
    const sweep = i => { const t = (i % 60) / 59, k = t < .5 ? t * 2 : 2 - t * 2; return [w.left + grid.width * (0.2 + 0.8 * k), g0.top]; };
    const size = await frames(120, i => { const [x, y] = sweep(i); fire('pointermove', window, x, y); });
    fire('pointerup', window, ...sweep(119));
    stats('cambiar tamaño', size);
  } catch (e) { say(false, 'exception: ' + e); }
  inv('qa_log', { line: 'DONE' });
})();
