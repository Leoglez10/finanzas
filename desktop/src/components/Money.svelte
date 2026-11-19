<script lang="ts">
  import { moneyParts } from "../lib/format";
  // A key figure: whole part large, cents smaller.
  let { cents, class: cls = "" }: { cents: number; class?: string } = $props();
  let parts = $derived(moneyParts(cents));
</script>
<p class="money {cls}">{parts.whole}<small>.{parts.decimals}</small></p>
