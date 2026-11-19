// Every change can be undone from the toast or with ⌘Z, so nothing asks "are you sure?".
export interface ToastAction { label: string; run: () => void }
export const toastState = $state({ message: "", action: null as ToastAction | null, visible: false });

let timer: ReturnType<typeof setTimeout> | undefined;
export function toast(message: string, action: ToastAction | null = null) {
  Object.assign(toastState, { message, action, visible: true });
  clearTimeout(timer);
  timer = setTimeout(() => { toastState.visible = false; }, action ? 6000 : 2400);
}

const stack: (() => Promise<unknown>)[] = [];
/** Call after a change succeeded; `revert` restores the previous state. */
export function remember(label: string, revert: () => Promise<unknown>) {
  stack.push(revert);
  toast(label, { label: "Deshacer", run: () => void undo() });
}
export async function undo() {
  const revert = stack.pop();
  if (!revert) return toast("Nada que deshacer");
  try { await revert(); toast("Deshecho"); } catch (e) { toast(`No se pudo deshacer: ${e}`); }
}
