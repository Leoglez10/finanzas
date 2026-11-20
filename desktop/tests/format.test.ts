// Run: pnpm --dir desktop test
import { test } from "node:test";
import assert from "node:assert/strict";
import { money, moneyParts } from "../src/lib/format.ts";

test("negative zero never shows a minus sign", () => {
  assert.equal(money(-0), "$0.00");
  assert.equal(money(-0, true), "$0.00");
  assert.deepEqual(moneyParts(-0), { whole: "$0", decimals: "00" });
});

test("signed amounts", () => {
  assert.equal(money(125050, true), "+$1,250.50");
  assert.equal(money(-8000), "-$80.00");
});
