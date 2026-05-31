<script>
  import { emitDipralixEvent } from '../../lib/events';

  let url = '';
  let isScanning = false;
  let stats = {
    files: 0,
    complexity: 'N/A',
    time: '0 min'
  };

  function handleScan() {
    if (!url) return;
    isScanning = true;
    emitDipralixEvent('scan', { url });
    
    // Simulate scan progress
    setTimeout(() => {
      isScanning = false;
      stats = {
        files: Math.floor(Math.random() * 500) + 50,
        complexity: ['Low', 'Medium', 'High'][Math.floor(Math.random() * 3)],
        time: `~${(Math.random() * 10).toFixed(1)} min`
      };
    }, 2000);
  }
</script>

<div class="glass p-8 rounded-2xl">
  <h2 class="text-3xl font-bold mb-4 tracking-tight">PROJECT DNA</h2>
  <p class="text-white/60 mb-8 text-sm">
    Paste a GitHub URL to visualize the architecture. DIPRALIX analyzes dependencies, complexity, and hotspots in real-time.
  </p>

  <div class="space-y-4">
    <div>
      <label class="block text-[10px] uppercase tracking-widest font-bold text-white/40 mb-2">Repository URL</label>
      <input 
        type="text" 
        bind:value={url}
        placeholder="https://github.com/pratikacharya1234/dipralix"
        class="w-full terminal-input text-sm py-3"
      />
    </div>
    <button 
      on:click={handleScan}
      disabled={isScanning}
      class="w-full btn-primary disabled:opacity-50 disabled:cursor-not-allowed"
    >
      {isScanning ? 'Scanning...' : 'Scan Repository'}
    </button>
  </div>

  <div class="mt-8 pt-8 border-t border-white/5 space-y-4">
    <div class="flex justify-between items-center">
      <span class="text-white/40 text-xs">Files Analyzed</span>
      <span class="font-mono text-sm">{stats.files}</span>
    </div>
    <div class="flex justify-between items-center">
      <span class="text-white/40 text-xs">Complexity Score</span>
      <span class="font-mono text-sm {stats.complexity === 'High' ? 'text-red-400' : stats.complexity === 'Medium' ? 'text-yellow-400' : 'text-green-400'}">{stats.complexity}</span>
    </div>
    <div class="flex justify-between items-center">
      <span class="text-white/40 text-xs">Estimated Task Time</span>
      <span class="font-mono text-sm">{stats.time}</span>
    </div>
  </div>
</div>