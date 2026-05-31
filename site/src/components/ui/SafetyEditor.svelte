<script>
  let policies = [
    { id: 'fs_read', name: 'Filesystem Read', status: 'allow', description: 'Allow reading files in project directory' },
    { id: 'fs_write', name: 'Filesystem Write', status: 'confirm', description: 'Ask before overwriting existing files' },
    { id: 'shell_exec', name: 'Shell Execution', status: 'confirm', description: 'Ask before running shell commands' },
    { id: 'network', name: 'Network Access', status: 'deny', description: 'Block all outbound network requests' }
  ];

  function toggleStatus(policy) {
    const states = ['allow', 'confirm', 'deny'];
    const currentIndex = states.indexOf(policy.status);
    policy.status = states[(currentIndex + 1) % states.length];
    policies = [...policies];
  }
</script>

<div class="glass rounded-2xl p-8 border-white/10">
  <div class="flex justify-between items-center mb-8">
    <div>
      <h3 class="text-xl font-bold mb-1">Safety Policy Editor</h3>
      <p class="text-xs text-white/40">Configure .dipralix/safety.toml visually</p>
    </div>
    <button class="btn-primary py-2 px-4 text-xs">Save Changes</button>
  </div>

  <div class="space-y-4">
    {#each policies as policy}
      <div class="flex items-center justify-between p-4 bg-white/5 rounded-xl border border-white/5 hover:border-white/10 transition-colors">
        <div>
          <div class="font-bold text-sm mb-1">{policy.name}</div>
          <div class="text-[10px] text-white/40">{policy.description}</div>
        </div>
        
        <button 
          on:click={() => toggleStatus(policy)}
          class="px-4 py-2 rounded-lg text-[10px] font-bold uppercase tracking-widest transition-all w-24 {policy.status === 'allow' || policy.status === 'confirm' ? 'text-black' : 'text-white'}"
          class:bg-gemini={policy.status === 'allow'}
          class:bg-gpt={policy.status === 'confirm'}
          class:bg-danger={policy.status === 'deny'}
        >
          {policy.status}
        </button>
      </div>
    {/each}
  </div>

  <div class="mt-8 pt-8 border-t border-white/5">
    <div class="flex items-center gap-2 text-[10px] font-mono text-white/20">
      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
      Changes will be written to .dipralix/safety.toml
    </div>
  </div>
</div>
