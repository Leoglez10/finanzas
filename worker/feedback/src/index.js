import { buildIssue } from "./issue.js";

const CORS = { "Access-Control-Allow-Origin": "*", "Access-Control-Allow-Methods": "POST, OPTIONS", "Access-Control-Allow-Headers": "Content-Type" };
const json = (data, status = 200) => new Response(JSON.stringify(data), { status, headers: { "Content-Type": "application/json", ...CORS } });

export default {
  async fetch(request, env) {
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
    return json({ number: created.number, url: created.html_url }, 201);
  },
};
