// Run: node design/redesign/quick-parse.test.cjs
const assert = require('node:assert/strict');
const { parseQuick, parseMoney } = require('./quick-parse.js');

const today = new Date(2026, 10, 10); // martes 10 nov 2026
const history = [
  { payee: 'Uber', cat: 'Transporte', acc: 'Cuenta principal' },
  { payee: 'Supermercado La Esquina', cat: 'Alimentación', acc: 'Cuenta principal' },
  { payee: 'Empresa SA', cat: 'Sueldo', acc: 'Cuenta principal' }
];
const ctx = { today, history, accounts: ['Cuenta principal', 'Tarjeta'] };
const day = d => [d.getFullYear(), d.getMonth() + 1, d.getDate()].join('-');

assert.equal(parseMoney('1,250.50'), 1250.5);
assert.equal(parseMoney('-80'), -80);
assert.equal(parseMoney('$3,200'), 3200);
assert.equal(parseMoney('1,2'), null);
assert.equal(parseMoney('12.345'), null);

let p = parseQuick('uber 120 ayer', ctx);
assert.deepEqual([p.kind, p.amount, p.payee, p.category, p.account, day(p.date)], ['gasto', 120, 'Uber', 'Transporte', 'Cuenta principal', '2026-11-9']);

p = parseQuick('super 450.50 tarjeta', ctx); // prefix matches history; explicit account wins
assert.deepEqual([p.amount, p.payee, p.category, p.account], [450.5, 'Supermercado La Esquina', 'Alimentación', 'Tarjeta']);

p = parseQuick('+3,200 sueldo', ctx);
assert.deepEqual([p.kind, p.amount, p.payee], ['ingreso', 3200, 'Sueldo']);

p = parseQuick('tacos 85 domingo', ctx); // keyword category, last Sunday
assert.deepEqual([p.payee, p.category, day(p.date)], ['Tacos', 'Alimentación', '2026-11-8']);

p = parseQuick('farmacia 99 5/11', ctx);
assert.deepEqual([p.amount, p.category, day(p.date)], [99, 'Salud', '2026-11-5']);

p = parseQuick('regalo para mamá', ctx); // no amount, no category: caller must ask
assert.deepEqual([p.amount, p.payee, p.category], [null, 'Regalo mamá', null]);

console.log('quick-parse: ok');
