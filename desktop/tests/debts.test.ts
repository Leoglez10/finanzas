// Run: pnpm --dir desktop test
import { test } from "node:test";
import assert from "node:assert/strict";
import { advice, cardForm, debtCandidates, cardTerms, emptyCardForm, missingText, parsePercent } from "../src/lib/debts.ts";
import type { DebtView } from "../src/lib/api.ts";

const debt = (over: Partial<DebtView>): DebtView => ({ accountId: 1, name: "Visa", isCard: true, terms: null, debt: 0, available: null, utilization: null, lastCut: null, nextCut: null, dueDate: null, overdue: false, statement: null, toAvoidInterest: null, minimum: null, interestIfMinimum: null, installments: [], missing: [], ...over });

test("every card field is optional", () => {
  assert.deepEqual(cardTerms(emptyCardForm()), { creditLimit: null, cutDay: null, dueDay: null, annualRate: null, minPayment: null });
});

test("card fields are read as cents, days and basis points and round-trip", () => {
  const terms = cardTerms({ limit: "25,000", cutDay: "5", dueDay: "25", rate: "45,5 %", minPayment: "10" });
  assert.deepEqual(terms, { creditLimit: 2_500_000, cutDay: 5, dueDay: 25, annualRate: 4550, minPayment: 1000 });
  assert.deepEqual(cardForm(terms as never), { limit: "25000.00", cutDay: "5", dueDay: "25", rate: "45.5", minPayment: "10" });
});

test("invalid card fields say what is wrong", () => {
  assert.match(cardTerms({ ...emptyCardForm(), cutDay: "32" }) as string, /1 al 31/);
  assert.match(cardTerms({ ...emptyCardForm(), limit: "0" }) as string, /límite/);
  assert.match(cardTerms({ ...emptyCardForm(), rate: "abc" }) as string, /tasa/);
  assert.match(cardTerms({ ...emptyCardForm(), minPayment: "150" }) as string, /mínimo/);
  assert.equal(parsePercent(""), null);
});

test("missing terms are named in Spanish", () => {
  assert.equal(missingText(debt({ missing: ["creditLimit", "cutDay", "annualRate"] })), "límite, día de corte y tasa");
});

test("advice puts overdue first, then what avoids interest, utilization and the highest rate", () => {
  const list = advice([
    debt({ name: "Visa", debt: 400_000, utilization: 0.8, toAvoidInterest: 250_000, dueDate: "2026-11-25", minimum: 30_000, interestIfMinimum: 15_660, terms: { creditLimit: 500_000, cutDay: 5, dueDay: 25, annualRate: 6000, minPayment: 1000 } }),
    debt({ name: "Amex", debt: 100_000, overdue: true, toAvoidInterest: 100_000, dueDate: "2026-11-20", terms: { creditLimit: null, cutDay: 1, dueDay: 20, annualRate: 3000, minPayment: null } }),
  ]);
  assert.deepEqual(list.map(a => a.tone), ["bad", "warn", "warn", "ok"]);
  assert.match(list[0].text, /Amex venció el 20 de noviembre/);
  assert.match(list[1].text, /Paga \$2,500\.00 de Visa antes del 25 de noviembre.*mínimo \(\$300\.00\).*\$156\.60/);
  assert.match(list[2].text, /80% del límite de Visa/);
  assert.match(list[3].text, /primero a Visa.*60%/);
  assert.deepEqual(advice([debt({ debt: 100 })]), []);
});

test("accounts in debt or named like a card suggest the Deudas page", () => {
  const acc = (name: string, balance = 0, closed = false) => ({ name, balance, closed });
  assert.deepEqual(debtCandidates([acc("Banco", 500), acc("Tarjeta Oro"), acc("TDC Nu"), acc("Crédito auto"), acc("Préstamo", -100), acc("Visa vieja", -5, true), acc("Ahorro")]).map(a => a.name),
    ["Tarjeta Oro", "TDC Nu", "Crédito auto", "Préstamo"]);
});
