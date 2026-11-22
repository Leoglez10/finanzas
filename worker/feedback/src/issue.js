// Validates a feedback report and turns it into a GitHub issue payload.
export const KINDS = { falla: { label: "bug", prefix: "Falla" }, mejora: { label: "enhancement", prefix: "Mejora" } };
export const LIMITS = { title: 120, body: 5000, meta: 60 };

const clean = (v, max) => (typeof v === "string" ? v.replace(/\r\n/g, "\n").trim() : "").slice(0, max);

/** Returns { issue } or { error } for an untrusted JSON body. */
export function buildIssue(input) {
  if (!input || typeof input !== "object") return { error: "Cuerpo inválido." };
  const kind = KINDS[input.kind];
  if (!kind) return { error: "Tipo inválido: usa falla o mejora." };
  const title = clean(input.title, LIMITS.title).replace(/\s+/g, " ");
  const body = clean(input.body, LIMITS.body);
  if (title.length < 4) return { error: "Escribe un título más claro." };
  if (body.length < 10) return { error: "Cuenta un poco más en la descripción." };
  const version = clean(input.version, LIMITS.meta) || "desconocida";
  const os = clean(input.os, LIMITS.meta) || "desconocido";
  return {
    issue: {
      title: `${kind.prefix}: ${title}`,
      // Quoted so reported text cannot ping people or close issues through keywords at the top level.
      body: `${body.split("\n").map(l => `> ${l}`).join("\n")}\n\n---\nEnviado desde la app · versión ${version} · ${os}`,
      labels: [kind.label, "desde-la-app"],
    },
  };
}
