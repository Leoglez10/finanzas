<script lang="ts">
  import Icon from "./Icon.svelte";
  import Money from "./Money.svelte";
  type Tone = "" | "ok" | "bad" | "warn" | "violet";
  // With `onclick` the card is a button that takes you to what the number is made of; `hint` says where.
  let { icon, tone = "", label, cents, value, note = "", onclick, hint = "" }: { icon: string; tone?: Tone; label: string; cents?: number; value?: string; note?: string; onclick?: () => void; hint?: string } = $props();
</script>
{#snippet body()}
  <div class="ic {tone}"><Icon name={icon} /></div>
  <div>
    <p>{label}</p>
    {#if cents !== undefined}<Money {cents} />{:else}<p class="money">{value}</p>{/if}
    {#if note}<p style="font-size:12px">{note}</p>{/if}
  </div>
{/snippet}
{#if onclick}
  <button type="button" class="card kpi link" title={hint} {onclick}>{@render body()}</button>
{:else}
  <article class="card kpi">{@render body()}</article>
{/if}
