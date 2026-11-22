// Sends a bug report or idea to the feedback Worker, which opens a GitHub issue.
// Only the typed text, the app version and the OS are sent; never data from the database.
export const FEEDBACK_URL = "https://finanzas-feedback.leoeligr10.workers.dev/report";
export const SECURITY_URL = "https://github.com/Leoglez10/finanzas/security/policy";

export type FeedbackKind = "falla" | "mejora";

export function osName(ua = navigator.userAgent): string {
  if (/Mac OS X|Macintosh/.test(ua)) return "macOS";
  if (/Windows/.test(ua)) return "Windows";
  if (/Linux/.test(ua)) return "Linux";
  return "desconocido";
}

export async function sendFeedback(report: { kind: FeedbackKind; title: string; body: string; version: string }): Promise<{ number: number; url: string }> {
  let res: Response;
  try {
    res = await fetch(FEEDBACK_URL, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ ...report, os: osName() }) });
  } catch {
    throw new Error("Sin conexión: no se pudo enviar el reporte.");
  }
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(data.error ?? `No se pudo enviar (error ${res.status}).`);
  return data;
}
