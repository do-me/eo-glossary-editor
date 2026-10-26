<script lang="ts">
  import { Minus, Plus } from '@lucide/svelte';
  let { zoom, onchange }: { zoom: number; onchange: (value: number) => void } = $props();
</script>

<div class="zoom-control" role="group" aria-label="Editor zoom">
  <button
    aria-label="Zoom out"
    title="Zoom out (Ctrl/Cmd −)"
    disabled={zoom <= 75}
    onclick={() => onchange(zoom - 10)}><Minus size={12} /></button
  >
  <input
    type="range"
    aria-label="Zoom"
    min="75"
    max="200"
    step="5"
    value={zoom}
    oninput={(e) => onchange(Number(e.currentTarget.value))}
  />
  <button
    aria-label="Zoom in"
    title="Zoom in (Ctrl/Cmd +)"
    disabled={zoom >= 200}
    onclick={() => onchange(zoom + 10)}><Plus size={12} /></button
  >
  <button
    class="zoom-value"
    aria-label="Reset zoom to 100%"
    title="Reset zoom (Ctrl/Cmd 0)"
    onclick={() => onchange(100)}>{zoom}%</button
  >
</div>
