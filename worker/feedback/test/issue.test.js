import { test } from "node:test";
import assert from "node:assert/strict";
import { buildIssue } from "../src/issue.js";

test("a bug report becomes a labeled, quoted issue", () => {
  const { issue } = buildIssue({ kind: "falla", title: "  No  guarda ", body: "Al guardar un gasto\nno aparece", version: "0.5.0", os: "macos" });
  assert.equal(issue.title, "Falla: No guarda");
  assert.deepEqual(issue.labels, ["bug", "desde-la-app"]);
  assert.match(issue.body, /^> Al guardar un gasto\n> no aparece\n\n---\nEnviado desde la app · versión 0.5.0 · macos$/);
});

test("an idea uses the enhancement label", () => {
  assert.deepEqual(buildIssue({ kind: "mejora", title: "Exportar CSV", body: "Quisiera exportar todo" }).issue.labels, ["enhancement", "desde-la-app"]);
});

test("invalid input is rejected", () => {
  assert.match(buildIssue(null).error, /inválido/);
  assert.match(buildIssue({ kind: "spam", title: "hola", body: "hola mundo!!" }).error, /Tipo/);
  assert.match(buildIssue({ kind: "falla", title: "a", body: "hola mundo!!" }).error, /título/);
  assert.match(buildIssue({ kind: "falla", title: "Título ok", body: "corto" }).error, /descripción/);
});

test("long fields are cut", () => {
  const { issue } = buildIssue({ kind: "falla", title: "x".repeat(500), body: "y".repeat(9000) });
  assert.equal(issue.title.length, "Falla: ".length + 120);
  assert.ok(issue.body.length < 5200);
});

test("the notification email names the issue and links it", async () => {
  const { notification } = await import("../src/mail.js");
  const issue = buildIssue({ kind: "falla", title: "Café no guarda", body: "Pasó algo raro" }).issue;
  assert.deepEqual(notification({ to: "yo@example.com", issue, number: 7, url: "https://x/7" }), {
    from: "Finanzas <onboarding@resend.dev>",
    to: ["yo@example.com"],
    subject: "#7 Falla: Café no guarda",
    text: "Falla: Café no guarda\nhttps://x/7\n\n> Pasó algo raro\n\n---\nEnviado desde la app · versión desconocida · desconocido\n",
  });
});
