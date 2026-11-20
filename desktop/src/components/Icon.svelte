<script lang="ts">
  // Symbols live in the sprite inside index.html.
  let { name }: { name: string } = $props();
</script>
<svg class="i" aria-hidden="true"><use href="#i-{name}" /></svg>
