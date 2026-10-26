<script lang="ts">
  // The mark, as assets/brand/logo.svg draws it (its generator is assets/brand/src): genoxide's
  // hexagon in iron-oxide colours, holding a silicon strip waveguide in its guided mode, the light
  // brightest in the core and fading into the oxide. The same in both themes, as the app's icon is.
  let { size = 28 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 128 128" aria-hidden="true">
  <defs>
    <linearGradient id="px-mark-face" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#2b1d17" /><stop offset="1" stop-color="#161010" /></linearGradient>
    <linearGradient id="px-mark-rim" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#ee8a32" /><stop offset="1" stop-color="#8a2a10" /></linearGradient>
    <radialGradient id="px-mark-mode" cx="0.5" cy="0.5" r="0.5"
      ><stop offset="0" stop-color="#f7b955" /><stop offset="0.35" stop-color="#ee8a32" stop-opacity="0.85" /><stop offset="0.7" stop-color="#b7410e" stop-opacity="0.4" /><stop offset="1" stop-color="#b7410e" stop-opacity="0" /></radialGradient
    >
    <linearGradient id="px-mark-core" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#f7b955" /><stop offset="0.5" stop-color="#fbe7c6" /><stop offset="1" stop-color="#f7b955" /></linearGradient>
  </defs>
  <path d="M64 6 L114.23 35 L114.23 93 L64 122 L13.77 93 L13.77 35 Z" fill="url(#px-mark-face)" stroke="url(#px-mark-rim)" stroke-width="6" stroke-linejoin="round" />
  <ellipse cx="64" cy="64" rx="47" ry="32" fill="url(#px-mark-mode)" />
  <g fill="none" stroke="#fbe7c6" stroke-width="2.5"><ellipse cx="64" cy="64" rx="30" ry="19" opacity="0.5" /><ellipse cx="64" cy="64" rx="39" ry="25.5" opacity="0.32" /><ellipse cx="64" cy="64" rx="47" ry="31.5" opacity="0.18" /></g>
  <rect x="41" y="55" width="46" height="18" rx="2.5" fill="url(#px-mark-core)" />
</svg>
