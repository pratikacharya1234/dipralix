<script>
  import { Canvas } from '@threlte/core';
  import DipralixScene from './DipralixScene.svelte';
</script>

<div class="w-full h-full">
  <Canvas>
    <DipralixScene />
    <div slot="fallback" class="w-full h-full flex items-center justify-center bg-background">
      <div class="text-white/20 font-mono text-xs uppercase tracking-widest">WebGL Not Supported</div>
    </div>
  </Canvas>
</div>
