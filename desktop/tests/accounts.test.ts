// Run: pnpm --dir desktop test
import { test } from "node:test";
import assert from "node:assert/strict";
import { ACCOUNT_COLORS, ACCOUNT_ICONS, accountColor, accountIcon } from "../src/lib/accounts.ts";

test("icon and color names match the ones finanzas-core accepts", () => {
  assert.deepEqual(ACCOUNT_ICONS.map(i => i[0]), ["bank", "card", "cash", "wallet", "piggy", "chart", "house", "car", "gift", "shield", "star", "plane"]);
  assert.deepEqual(ACCOUNT_COLORS.map(c => c[0]), ["violet", "blue", "teal", "green", "lime", "yellow", "orange", "red", "pink", "gray"]);
});

test("accounts without a chosen look keep the previous icon; colors map to CSS", () => {
  assert.equal(accountIcon({ offBudget: true, icon: null }), "chart");
  assert.equal(accountIcon({ offBudget: false, icon: null }), "card");
  assert.equal(accountIcon({ offBudget: false, icon: "piggy" }), "piggy");
  assert.equal(accountColor("blue"), "#3B82F6");
  assert.equal(accountColor(null), null);
  assert.equal(accountColor("neon"), null);
});
