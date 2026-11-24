<script lang="ts">
  // An account's own image, or its icon on its color. Images come as data: URLs and are drawn with <img>, so an SVG cannot run scripts.
  import { accountColor, accountIcon, type AccountLook } from "../lib/accounts";
  import Icon from "./Icon.svelte";
  let { account, size = 24 }: { account: AccountLook; size?: number } = $props();
  let color = $derived(accountColor(account.color));
</script>
{#if account.image}
  <img class="badge img" src={account.image} alt="" style="--s:{size}px" />
{:else}
  <span class="badge" class:tinted={color} style="--s:{size}px;{color ? `--bc:${color}` : ''}" aria-hidden="true"><Icon name={accountIcon(account)} /></span>
{/if}

<style>
  .badge { width: var(--s); height: var(--s); border-radius: calc(var(--s) * .28); display: inline-grid; place-items: center; flex: none; color: currentColor; }
  .badge :global(svg.i) { width: calc(var(--s) * .72); height: calc(var(--s) * .72); }
  .tinted { background: color-mix(in srgb, var(--bc) 16%, transparent); color: var(--bc); }
  .img { object-fit: contain; background: #fff; border: 1px solid var(--border); }
</style>
