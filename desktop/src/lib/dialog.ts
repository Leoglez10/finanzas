export function dialogFocus(node: HTMLElement) {
  const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  const controls = () => Array.from(node.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex="0"]'));
  queueMicrotask(() => (controls()[0] ?? node).focus());
  const keydown = (event: KeyboardEvent) => {
    if (event.key !== 'Tab') return;
    const items = controls(); if (!items.length) { event.preventDefault(); return; }
    const first = items[0], last = items[items.length - 1];
    if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
    else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
  };
  node.addEventListener('keydown', keydown);
  return { destroy() { node.removeEventListener('keydown', keydown); previous?.focus(); } };
}
