<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  // Backend URL — overridable at build time via PUBLIC_STATS_URL
  const STATS_URL = (import.meta as any).env?.PUBLIC_STATS_URL || 'https://dipralix-stats.fly.dev/api/stats';
  const POLL_MS = 5000;

  type Stats = {
    stars: number;
    forks: number;
    downloads: number;
    visits: number;
    as_of: string;
  };

  let stats: Stats = { stars: 0, forks: 0, downloads: 0, visits: 0, as_of: '' };
  let displayed = { stars: 0, forks: 0, downloads: 0, visits: 0 };
  let connected = false;
  let lastError: string | null = null;
  let pollTimer: ReturnType<typeof setInterval> | null = null;
  let tickTimer: ReturnType<typeof setInterval> | null = null;

  async function fetchStats() {
    try {
      const res = await fetch(STATS_URL, { cache: 'no-store' });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      stats = await res.json();
      connected = true;
      lastError = null;
    } catch (e: any) {
      connected = false;
      lastError = e?.message ?? 'unknown error';
    }
  }

  // Animate displayed numbers toward actual stats — gives the "live ticking" feel.
  function tick() {
    (['stars', 'forks', 'downloads', 'visits'] as const).forEach((k) => {
      const target = stats[k];
      const current = displayed[k];
      const diff = target - current;
      if (diff === 0) return;
      // Move ~1/8 of the gap per tick, but always at least 1.
      const step = diff > 0
        ? Math.max(1, Math.ceil(diff / 8))
        : Math.min(-1, Math.floor(diff / 8));
      displayed[k] = current + step;
      // Snap on overshoot.
      if ((diff > 0 && displayed[k] > target) || (diff < 0 && displayed[k] < target)) {
        displayed[k] = target;
      }
    });
    displayed = displayed;
  }

  function formatNumber(n: number): string {
    return n.toLocaleString('en-US');
  }

  onMount(() => {
    fetchStats();
    pollTimer = setInterval(fetchStats, POLL_MS);
    tickTimer = setInterval(tick, 80);
  });

  onDestroy(() => {
    if (pollTimer) clearInterval(pollTimer);
    if (tickTimer) clearInterval(tickTimer);
  });
</script>

<div class="grid grid-cols-2 md:grid-cols-4 gap-3">
  {#each [
    { key: 'stars',     label: 'Stars',     hint: 'GitHub stargazers' },
    { key: 'forks',     label: 'Forks',     hint: 'GitHub forks' },
    { key: 'downloads', label: 'Downloads', hint: 'Release asset downloads' },
    { key: 'visits',    label: 'Visits',    hint: 'Site visits this deploy' },
  ] as item}
    <div class="p-5 rounded-xl border border-white/5 bg-white/[0.02] hover:bg-white/[0.04] transition-colors relative overflow-hidden">
      <div class="flex items-center gap-2 mb-2">
        <span class="w-1.5 h-1.5 rounded-full {connected ? 'bg-green-400 animate-pulse' : 'bg-red-400'}"></span>
        <div class="text-xs uppercase tracking-widest text-white/40">{item.label}</div>
      </div>
      <div class="text-3xl md:text-4xl font-bold tracking-tighter tabular-nums text-white">
        {formatNumber(displayed[item.key])}
      </div>
      <div class="text-xs text-white/30 mt-1">{item.hint}</div>
    </div>
  {/each}
</div>

<div class="mt-4 text-center">
  {#if connected}
    <span class="text-xs text-white/30">Live · refreshes every {POLL_MS / 1000}s · powered by a Rust backend on Fly.io</span>
  {:else}
    <span class="text-xs text-red-400/60">Disconnected — {lastError ?? 'no response'} (retrying)</span>
  {/if}
</div>
