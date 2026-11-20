<script lang="ts">
  import { toastState } from "../lib/undo.svelte";
</script>
<div class="toast" class:show={toastState.visible} role="status" aria-live="polite">
  <span>{toastState.message}</span>
  {#if toastState.action}
    <button type="button" onclick={() => { toastState.visible = false; toastState.action?.run(); }}>{toastState.action.label}</button>
  {/if}
</div>
