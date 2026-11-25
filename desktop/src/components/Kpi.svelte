<script lang="ts">
  import Money from "./Money.svelte";
  type Tone = "" | "ok" | "bad" | "warn" | "violet";
  // One cell of a `.kpis` strip: label, big number, optional note. `tone` colors the number only for
  // money in (ok) and money out (bad). With `onclick` the cell is a button that takes you to what the
  // number is made of; `hint` says where.
  let { tone = "", label, cents, value, note = "", onclick, hint = "" }: { tone?: Tone; label: string; cents?: number; value?: string; note?: string; onclick?: () => void; hint?: string } = $props();
</script>
{#snippet body()}
  <p>{label}</p>
  {#if cents !== undefined}<Money {cents} />{:else}<p class="money">{value}</p>{/if}
  {#if note}<p class="note">{note}</p>{/if}
{/snippet}
{#if onclick}
  <button type="button" class="kpi link {tone}" title={hint} {onclick}>{@render body()}</button>
{:else}
  <article class="kpi {tone}">{@render body()}</article>
{/if}
