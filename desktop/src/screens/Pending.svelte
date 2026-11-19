<script lang="ts">
  // Temporary stand-in while the screen is being ported from the prototype (odd/tasks/tauri-app.md).
  import Topbar from "../components/Topbar.svelte";
  let { title }: { title: string } = $props();
</script>
<Topbar {title} />
<article class="card"><p class="muted">Esta pantalla está en construcción. Mientras tanto sigue disponible en la app actual.</p></article>
