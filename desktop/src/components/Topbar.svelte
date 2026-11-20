<script lang="ts">
  import type { Snippet } from "svelte";
  let { title, sub = "", lead, actions }: { title: string; sub?: string; lead?: Snippet; actions?: Snippet } = $props();
</script>
<header class="topbar" data-tauri-drag-region>
  {@render lead?.()}
  <div>{#if sub}<p class="sub">{sub}</p>{/if}<h1>{title}</h1></div>
  <div class="actions">{@render actions?.()}</div>
</header>
