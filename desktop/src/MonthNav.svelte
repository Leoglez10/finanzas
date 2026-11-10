<script lang="ts">
 let {month = $bindable(), currentMonth}: {month:number; currentMonth:number} = $props();
 function step(delta:number) { const d = new Date(Math.floor(month/100), month%100-1+delta, 1); month = d.getFullYear()*100+d.getMonth()+1; }
 function key(e:KeyboardEvent) { if ((e.metaKey || e.ctrlKey) && !e.altKey && (e.key === 'ArrowLeft' || e.key === 'ArrowRight')) { e.preventDefault(); step(e.key === 'ArrowLeft' ? -1 : 1); } }
</script>
<svelte:window onkeydown={key}/>
<div><button onclick={() => step(-1)} title="⌘/Ctrl+←">Mes anterior</button><button onclick={() => month = currentMonth}>Hoy</button><button onclick={() => step(1)} title="⌘/Ctrl+→">Mes siguiente</button></div>
<style>div{display:flex;gap:8px;flex-wrap:wrap}button{font:inherit;color:inherit;background:var(--canvas);border:1px solid var(--border);border-radius:8px;padding:7px;cursor:pointer}</style>
