// Resend email payload for a new issue. Without a verified domain Resend only delivers from
// onboarding@resend.dev to the Resend account's own address, which is all this needs.
export function notification({ to, issue, number, url }) {
  return {
    from: "Finanzas <onboarding@resend.dev>",
    to: [to],
    subject: `#${number} ${issue.title}`,
    text: `${issue.title}\n${url}\n\n${issue.body}\n`,
  };
}
