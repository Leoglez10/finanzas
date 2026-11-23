<script lang="ts">
  // Credit-card terms: every field optional. Used when creating an account and when editing a card.
  import type { CardForm } from "../lib/debts";
  let { form = $bindable() }: { form: CardForm } = $props();
  let incomplete = $derived(Object.values(form).some(v => !v.trim()));
</script>

<div class="two">
  <label class="field"><span>Límite de crédito <span class="opt">· opcional</span></span><input class="input num" bind:value={form.limit} inputmode="decimal" placeholder="25,000" /></label>
  <label class="field"><span>Tasa anual <span class="opt">· opcional</span></span><input class="input num" bind:value={form.rate} inputmode="decimal" placeholder="45.5 %" aria-describedby="card-rate-hint" />
    <small id="card-rate-hint">En tu estado de cuenta: “tasa de interés anual ordinaria”.</small></label>
</div>
<div class="three">
  <label class="field"><span>Día de corte <span class="opt">· opcional</span></span><input class="input num" bind:value={form.cutDay} inputmode="numeric" placeholder="5" /></label>
  <label class="field"><span>Día límite de pago <span class="opt">· opcional</span></span><input class="input num" bind:value={form.dueDay} inputmode="numeric" placeholder="25" /></label>
  <label class="field"><span>Pago mínimo <span class="opt">· opcional</span></span><input class="input num" bind:value={form.minPayment} inputmode="decimal" placeholder="10 %" /></label>
</div>
{#if incomplete}<p class="card-note" role="note">Todo es opcional. Si agregas el límite, las fechas y la tasa, la app puede recomendarte cuánto pagar y cómo evitar intereses.</p>{/if}

<style>
  .three { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; }
  .card-note { margin: 0; padding: 10px 12px; border-radius: 10px; background: var(--brand-soft); color: var(--brand); font-size: 13px; }
</style>
