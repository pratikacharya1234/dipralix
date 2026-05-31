<script>
  import { onMount } from 'svelte';
  import { emitDipralixEvent } from '../../lib/events';

  let input = '';
  let history = [
    { type: 'system', content: 'DIPRALIX v0.0.1 initialized.' },
    { type: 'system', content: 'Connected to Gemini 2.5 Flash, Claude 4 Sonnet, GPT-4.1' }
  ];

  function handleSubmit() {
    if (!input.trim()) return;
    
    const task = input;
    history = [...history, { type: 'user', content: task }];
    input = '';
    
    emitDipralixEvent('input', { text: task });

    // Simulate DIPRALIX response
    setTimeout(() => {
      history = [...history, { type: 'dipralix', content: 'Classification: Code Change' }];
      emitDipralixEvent('process', { step: 'classification' });
    }, 500);

    setTimeout(() => {
      history = [...history, { type: 'dipralix', content: 'Planning: Analyzing project structure...' }];
      emitDipralixEvent('process', { step: 'planning' });
    }, 1200);

    setTimeout(() => {
      history = [...history, { type: 'dipralix', content: 'Reading site/src/pages/index.astro...' }];
      emitDipralixEvent('spark', { color: '#6366f1' });
    }, 2000);

    setTimeout(() => {
      history = [...history, { type: 'dipralix', content: 'SUCCESS: Task completed.' }];
      emitDipralixEvent('success');
    }, 3000);
  }

  let terminalEl;
  $: if (history && terminalEl) {
    setTimeout(() => {
      terminalEl.scrollTop = terminalEl.scrollHeight;
    }, 0);
  }
</script>

<div class="glass rounded-xl overflow-hidden shadow-2xl border-white/10 flex flex-col h-64">
  <div class="bg-white/5 px-4 py-2 flex items-center gap-2 border-b border-white/5">
    <div class="flex gap-1.5">
      <div class="w-2.5 h-2.5 rounded-full bg-red-500/50"></div>
      <div class="w-2.5 h-2.5 rounded-full bg-yellow-500/50"></div>
      <div class="w-2.5 h-2.5 rounded-full bg-green-500/50"></div>
    </div>
    <div class="text-[10px] uppercase tracking-widest text-white/30 font-bold ml-2">Dipralix Terminal — Session #0812</div>
  </div>
  
  <div 
    bind:this={terminalEl}
    class="flex-1 p-4 font-mono text-sm overflow-y-auto space-y-2 scroll-smooth"
  >
    {#each history as line}
      <div class="flex gap-3">
        {#if line.type === 'user'}
          <span class="text-accent-primary">❯</span>
          <span class="text-white">{line.content}</span>
        {:else if line.type === 'system'}
          <span class="text-white/30">#</span>
          <span class="text-white/40 italic">{line.content}</span>
        {:else}
          <span class="text-accent-secondary">dipralix</span>
          <span class="text-white/80">{line.content}</span>
        {/if}
      </div>
    {/each}
  </div>

  <form on:submit|preventDefault={handleSubmit} class="p-4 bg-black/20 border-t border-white/5 flex gap-3">
    <span class="text-accent-primary font-mono">❯</span>
    <input 
      bind:value={input}
      placeholder="Type a task (e.g. 'Build a 3D dipralix scene')"
      class="bg-transparent border-none outline-none flex-1 font-mono text-sm text-white placeholder:text-white/20"
    />
  </form>
</div>
