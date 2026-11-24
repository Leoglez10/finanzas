<script lang="ts">
  import { ACCOUNT_COLORS, ACCOUNT_ICONS } from "../lib/accounts";
  import Icon from "./Icon.svelte";
  let { icon = $bindable(), color = $bindable() }: { icon: string | null; color: string | null } = $props();
</script>

<div class="field">Icono
  <div class="icons" role="radiogroup" aria-label="Icono de la cuenta">
    {#each ACCOUNT_ICONS as [id, label] (id)}
      <button type="button" role="radio" aria-checked={icon === id} aria-label={label} title={label} class:on={icon === id} onclick={() => icon = icon === id ? null : id}><Icon name={id} /></button>
    {/each}
  </div>
</div>
<div class="field">Color
  <div class="colors" role="radiogroup" aria-label="Color de la cuenta">
    {#each ACCOUNT_COLORS as [id, label, css] (id)}
      <button type="button" role="radio" aria-checked={color === id} aria-label={label} title={label} class:on={color === id} style="--sw:{css}" onclick={() => color = color === id ? null : id}></button>
    {/each}
  </div>
</div>

<style>
  .icons { display: grid; grid-template-columns: repeat(6, 1fr); gap: 6px; margin-top: 6px; }
  .icons button { height: 38px; border-radius: 10px; border: 1px solid var(--border); background: var(--surface); color: var(--ink); cursor: pointer; display: grid; place-items: center; }
  .icons button.on { border-color: var(--brand); background: var(--brand-soft); color: var(--brand); }
  .colors { display: flex; gap: 8px; flex-wrap: wrap; margin-top: 6px; }
  .colors button { width: 26px; height: 26px; border-radius: 50%; border: 2px solid var(--surface); background: var(--sw); cursor: pointer; box-shadow: 0 0 0 1px var(--border); }
  .colors button.on { box-shadow: 0 0 0 2px var(--ink); }
  button:focus-visible { outline: 2px solid var(--brand); outline-offset: 2px; }
</style>
