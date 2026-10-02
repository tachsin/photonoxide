<script lang="ts">
  // The mark: a beam bent through a prism-like guide.
  let { size = 28 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 32 32" aria-hidden="true">
  <defs>
    <linearGradient id="px-bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="var(--color-primary)" />
      <stop offset="1" stop-color="var(--color-secondary)" />
    </linearGradient>
  </defs>
  <rect x="1" y="1" width="30" height="30" rx="8" fill="url(#px-bg)" />
  <path d="M5 21 L13 17 L27 9" stroke="white" stroke-width="2.4" stroke-linecap="round" fill="none" opacity=".95" />
  <path d="M13 17 L27 15" stroke="white" stroke-width="1.6" stroke-linecap="round" fill="none" opacity=".55" />
  <path d="M13 17 L27 21" stroke="white" stroke-width="1.2" stroke-linecap="round" fill="none" opacity=".35" />
  <circle cx="13" cy="17" r="2.4" fill="white" />
</svg>
