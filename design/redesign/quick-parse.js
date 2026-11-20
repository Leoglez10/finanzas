// Quick capture parser: turns "uber 120 ayer" into a movement draft.
// Classic script (works from file:// and in a Tauri webview); also loadable from Node for tests.
(function (root) {
  const norm = s => String(s).normalize('NFD').replace(/\p{Diacritic}/gu, '').toLowerCase();
  const MONEY = /^([+-])?\$?((\d{1,3}(,\d{3})+|\d+)(\.\d{1,2})?)$/;
  const WEEKDAYS = ['domingo', 'lunes', 'martes', 'miercoles', 'jueves', 'viernes', 'sabado'];
  const INCOME = ['ingreso', 'cobre', 'cobro', 'sueldo', 'nomina', 'reembolso', 'deposito'];
  const FILLER = ['de', 'en', 'a', 'para', 'por', 'con', 'el', 'la', 'los', 'las', 'hoy'];
  // ponytail: fixed keyword list; learn from the user's own history first (below), extend when misses show up.
  const KEYWORDS = {
    Transporte: ['uber', 'didi', 'gasolina', 'gasolinera', 'taxi', 'metro', 'camion', 'estacionamiento'],
    'Alimentación': ['super', 'supermercado', 'comida', 'tacos', 'restaurante', 'cafe', 'oxxo', 'despensa'],
    Salud: ['farmacia', 'doctor', 'medicina', 'dentista'],
    Servicios: ['cfe', 'luz', 'agua', 'internet', 'telefono', 'gas'],
    Ocio: ['netflix', 'spotify', 'cine', 'juego', 'concierto'],
    Vivienda: ['renta', 'casa', 'mantenimiento']
  };

  /** "1,250.50" → 1250.5, "-80" → -80, "1,2" → null. Only comma-thousands and period-decimals. */
  function parseMoney(raw) {
    const m = String(raw).trim().match(MONEY);
    if (!m) return null;
    const v = parseFloat(m[2].replace(/,/g, ''));
    return m[1] === '-' ? -v : v;
  }

  const addDays = (d, n) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);

  /**
   * @param {string} text  free text, e.g. "super 450.50 tarjeta"
   * @param {{today: Date, history?: {payee:string, cat:string, acc:string}[], accounts?: string[]}} ctx
   *        history is most-recent-first; it decides payee spelling, category and account.
   */
  function parseQuick(text, { today, history = [], accounts = [] }) {
    const out = { kind: 'gasto', amount: null, payee: '', category: null, account: null, date: addDays(today, 0) };
    const words = [];
    for (const raw of String(text).trim().split(/\s+/).filter(Boolean)) {
      const w = norm(raw);
      const money = out.amount === null ? parseMoney(raw) : null;
      if (money !== null) { out.amount = Math.abs(money); if (raw.startsWith('+')) out.kind = 'ingreso'; continue; }
      if (w === 'ayer') { out.date = addDays(today, -1); continue; }
      if (w === 'antier' || w === 'anteayer') { out.date = addDays(today, -2); continue; }
      const wd = WEEKDAYS.indexOf(w);
      if (wd >= 0) { out.date = addDays(today, -((today.getDay() - wd + 7) % 7)); continue; }
      const dm = raw.match(/^(\d{1,2})\/(\d{1,2})(?:\/(\d{4}))?$/);
      if (dm) { out.date = new Date(dm[3] ? +dm[3] : today.getFullYear(), +dm[2] - 1, +dm[1]); continue; }
      if (INCOME.includes(w)) { out.kind = 'ingreso'; if (w === 'sueldo' || w === 'nomina') words.push(raw); continue; }
      const acc = accounts.find(a => norm(a) === w);
      if (acc) { out.account = acc; continue; }
      if (FILLER.includes(w)) continue;
      words.push(raw);
    }
    const p = norm(words.join(' '));
    const past = p && (history.find(h => norm(h.payee) === p) || history.find(h => norm(h.payee).startsWith(p)));
    if (past) {
      out.payee = past.payee; out.category = past.cat;
      if (!out.account) out.account = past.acc;
    } else {
      out.payee = p ? words.join(' ').replace(/^./, c => c.toUpperCase()) : '';
      const tokens = p.split(' ');
      out.category = Object.keys(KEYWORDS).find(cat => KEYWORDS[cat].some(k => tokens.includes(k))) || null;
    }
    if (out.category === 'Sueldo') out.kind = 'ingreso';
    return out;
  }

  const api = { parseQuick, parseMoney, norm };
  if (typeof module !== 'undefined') module.exports = api; else Object.assign(root, api);
})(this);
