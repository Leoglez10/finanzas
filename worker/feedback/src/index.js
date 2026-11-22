import { buildIssue } from "./issue.js";
import { notification } from "./mail.js";

const CORS = { "Access-Control-Allow-Origin": "*", "Access-Control-Allow-Methods": "POST, OPTIONS", "Access-Control-Allow-Headers": "Content-Type" };
const json = (data, status = 200) => new Response(JSON.stringify(data), { status, headers: { "Content-Type": "application/json", ...CORS } });

export default {
  async fetch(request, env, ctx) {
    if (request.method === "OPTIONS") return new Response(null, { status: 204, headers: CORS });
    if (request.method !== "POST" || new URL(request.url).pathname !== "/report") return json({ error: "No encontrado." }, 404);
    const ip = request.headers.get("CF-Connecting-IP") || "unknown";
    const { success } = await env.LIMITER.limit({ key: ip });
    if (!success) return json({ error: "Demasiados reportes seguidos. Intenta en un minuto." }, 429);
    if (Number(request.headers.get("Content-Length") || 0) > 16_000) return json({ error: "El reporte es demasiado largo." }, 413);
    let input;
    try { input = await request.json(); } catch { return json({ error: "Cuerpo inválido." }, 400); }
    const { issue, error } = buildIssue(input);
    if (error) return json({ error }, 400);
    const res = await fetch(`https://api.github.com/repos/${env.REPO}/issues`, {
      method: "POST",
      headers: { Authorization: `Bearer ${env.GITHUB_TOKEN}`, Accept: "application/vnd.github+json", "User-Agent": "finanzas-feedback", "X-GitHub-Api-Version": "2022-11-28" },
      body: JSON.stringify(issue),
    });
    if (!res.ok) { console.error("github", res.status, await res.text()); return json({ error: "No se pudo crear el reporte. Intenta más tarde." }, 502); }
    const created = await res.json();
    // The issue is authored by the token owner, so GitHub sends no email; notify directly. A failed email never fails the report.
    if (env.RESEND_API_KEY && env.NOTIFY_EMAIL) {
      ctx.waitUntil(fetch("https://api.resend.com/emails", {
        method: "POST",
        headers: { Authorization: `Bearer ${env.RESEND_API_KEY}`, "Content-Type": "application/json" },
        body: JSON.stringify(notification({ to: env.NOTIFY_EMAIL, issue, number: created.number, url: created.html_url })),
      }).then(async r => { if (!r.ok) console.error("email", r.status, await r.text()); }).catch(e => console.error("email", e)));
    }
    return json({ number: created.number, url: created.html_url }, 201);
  },
};
