<script lang="ts">
  import Icon from "./Icon.svelte";
  import Money from "./Money.svelte";
  type Tone = "" | "ok" | "bad" | "warn" | "violet";
  let { icon, tone = "", label, cents, value, note = "" }: { icon: string; tone?: Tone; label: string; cents?: number; value?: string; note?: string } = $props();
</script>
<article class="card kpi">
  <div class="ic {tone}"><Icon name={icon} /></div>
  <div>
    <p>{label}</p>
    {#if cents !== undefined}<Money {cents} />{:else}<p class="money">{value}</p>{/if}
    {#if note}<p style="font-size:12px">{note}</p>{/if}
  </div>
</article>
