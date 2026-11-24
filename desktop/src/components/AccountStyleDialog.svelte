<script lang="ts">
  // Icon, color and own image of one account. The image applies right away; icon and color on Guardar.
  import { untrack } from "svelte";
  import * as api from "../lib/api";
  import { app, refresh } from "../lib/app.svelte";
  import { accountStyleDialog } from "../lib/dialogs.svelte";
  import { remember, toast } from "../lib/undo.svelte";
  import AccountBadge from "./AccountBadge.svelte";
  import Icon from "./Icon.svelte";
  import StylePicker from "./StylePicker.svelte";

  let dialog: HTMLDialogElement;
  let icon = $state<string | null>(null), color = $state<string | null>(null), busy = $state(false), error = $state("");
  let account = $derived(app.accounts.find(a => a.id === accountStyleDialog.accountId));
  $effect(() => { if (accountStyleDialog.open) untrack(() => {
    icon = account?.icon ?? null; color = account?.color ?? null; error = ""; busy = false;
    if (!dialog.open) dialog.showModal();
  }); });

  async function image(run: () => Promise<unknown>) {
    error = ""; busy = true;
    try { await run(); await refresh(); } catch (e) { error = String(e); } finally { busy = false; }
  }
  async function submit(e: SubmitEvent) {
    if ((e.submitter as HTMLButtonElement | null)?.value !== "save" || !account) return;
    e.preventDefault();
    const before = { icon: account.icon, color: account.color }, id = account.id, name = account.name;
    busy = true;
    try {
      await api.setAccountStyle(id, icon, color);
      remember(`Apariencia de ${name} guardada`, () => api.setAccountStyle(id, before.icon, before.color).then(refresh));
      dialog.close(); await refresh();
    } catch (err) { toast(String(err)); } finally { busy = false; }
  }
</script>

<dialog bind:this={dialog} aria-labelledby="style-title" onclose={() => accountStyleDialog.open = false}>
  <form method="dialog" novalidate onsubmit={submit}>
    <div class="dlg-h"><div><h2 id="style-title">Apariencia</h2><p>{account?.name ?? ""}</p></div>
      <button class="btn icon ghost" type="button" onclick={() => dialog.close()} aria-label="Cerrar"><Icon name="x" /></button></div>
    {#if account}
      <div class="dlg-b">
        <div class="preview"><AccountBadge account={{ ...account, icon, color, image: account.image }} size={44} /><b>{account.name}</b></div>
        <StylePicker bind:icon bind:color />
        <div class="field">Imagen propia <span class="opt">· opcional, se guarda solo en tus datos</span>
          <div class="img-row">
            <button class="btn sm" type="button" disabled={busy} onclick={() => image(() => api.pickAccountImage(account!.id))}><Icon name="upload" />{account.image ? "Cambiar imagen" : "Elegir imagen…"}</button>
            {#if account.image}<button class="btn sm ghost" type="button" disabled={busy} onclick={() => image(() => api.clearAccountImage(account!.id))}>Quitar imagen</button>{/if}
          </div>
          <small>PNG, JPG, WebP o SVG de hasta 512 KB. Con imagen, el icono queda de respaldo.</small>
        </div>
        {#if error}<p class="err" role="alert">{error}</p>{/if}
      </div>
    {/if}
    <div class="dlg-f"><button class="btn" type="button" onclick={() => dialog.close()}>Cancelar</button><button class="btn primary" value="save" disabled={busy}>Guardar</button></div>
  </form>
</dialog>

<style>
  .preview { display: flex; align-items: center; gap: 12px; font-size: 16px; }
  .img-row { display: flex; gap: 8px; margin-top: 6px; }
</style>
