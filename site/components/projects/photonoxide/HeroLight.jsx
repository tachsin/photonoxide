"use client";

import { useEffect, useRef } from "react";

/**
 * The hero's backdrop: a wavelength demultiplexer seen from above. Two
 * wavelengths share one waveguide on the left, cross a freeform design
 * region, and leave by separate outputs on the right. Decorative (hidden
 * from assistive technology, no pointer events); under reduced motion one
 * still frame. It runs only while on screen in a visible tab. It is a band
 * of its own under the hero's text, full width.
 */

const TAU = Math.PI * 2;

// the design region's pixels: a fixed pattern, so every visitor sees the same device
const CELLS = 14;
const PATTERN = Array.from({ length: CELLS * CELLS }, (_, k) => {
  const i = k % CELLS;
  const j = Math.floor(k / CELLS);
  const v = Math.sin(i * 1.7 + j * 0.9) + Math.cos(i * 0.6 - j * 1.3) + Math.sin((i + j) * 0.45);
  return v > 0.35;
});

function readColors(canvas) {
  const style = getComputedStyle(canvas);
  const ink = style.getPropertyValue("--color-base-content").trim() || "#1f2937";
  const base = style.getPropertyValue("--color-base-100").trim() || "#ffffff";
  // dark when the base is dark: probe through the canvas itself
  const probe = document.createElement("canvas").getContext("2d");
  probe.fillStyle = base;
  probe.fillRect(0, 0, 1, 1);
  const [r, g, b] = probe.getImageData(0, 0, 1, 1).data;
  const dark = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255 < 0.45;
  return {
    ink,
    dark,
    // 1310 nm and 1550 nm as two hues: neither is visible light
    short: dark ? "oklch(0.78 0.14 235)" : "oklch(0.55 0.16 245)",
    long: dark ? "oklch(0.76 0.15 35)" : "oklch(0.6 0.17 32)",
  };
}

/** The device's geometry for a canvas of w x h CSS pixels. */
function geometry(w, h) {
  const size = Math.min(h * 0.8, w * 0.28, 170);
  const cy = h * 0.5;
  const x0 = w / 2 - size / 2;
  const x1 = w / 2 + size / 2;
  const split = size * 0.32;
  return { w, h, size, cy, x0, x1, split, amp: Math.max(4, size * 0.05) };
}

/** A point along wavelength c's path (0 up, 1 down) at x. */
function pathY(g, x, c) {
  if (x <= g.x0) return g.cy;
  const sign = c === 0 ? -1 : 1;
  if (x >= g.x1) return g.cy + sign * g.split;
  const t = (x - g.x0) / (g.x1 - g.x0);
  const s = t * t * (3 - 2 * t); // smoothstep through the design region
  return g.cy + sign * g.split * s;
}

function draw(ctx, g, colors, time) {
  const { w, h } = g;
  ctx.clearRect(0, 0, w, h);
  ctx.lineCap = "round";

  // waveguides: the input, the two outputs, faint
  ctx.strokeStyle = colors.ink;
  ctx.globalAlpha = colors.dark ? 0.16 : 0.12;
  ctx.lineWidth = g.amp * 2.6;
  ctx.beginPath();
  ctx.moveTo(-10, g.cy);
  ctx.lineTo(g.x0, g.cy);
  for (const sign of [-1, 1]) {
    ctx.moveTo(g.x1, g.cy + sign * g.split);
    ctx.lineTo(w + 10, g.cy + sign * g.split);
  }
  ctx.stroke();

  // the design region's silicon pixels
  const cell = g.size / CELLS;
  ctx.fillStyle = colors.ink;
  ctx.globalAlpha = colors.dark ? 0.1 : 0.07;
  for (let k = 0; k < PATTERN.length; k++) {
    if (!PATTERN[k]) continue;
    const i = k % CELLS;
    const j = Math.floor(k / CELLS);
    ctx.fillRect(g.x0 + i * cell + 0.5, g.cy - g.size / 2 + j * cell + 0.5, cell - 1, cell - 1);
  }
  ctx.globalAlpha = colors.dark ? 0.22 : 0.16;
  ctx.lineWidth = 1;
  ctx.strokeRect(g.x0, g.cy - g.size / 2, g.size, g.size);

  // the two waves: shorter period for 1310 nm, each fading in from the left edge
  const waves = [
    { color: colors.short, period: g.size * 0.21, speed: 1.0, c: 0 },
    { color: colors.long, period: g.size * 0.25, speed: 1.0, c: 1 },
  ];
  ctx.lineWidth = 1.6;
  for (const wave of waves) {
    ctx.strokeStyle = wave.color;
    ctx.beginPath();
    for (let x = 0; x <= w; x += 2) {
      const y = pathY(g, x, wave.c);
      const phase = (TAU * x) / wave.period - time * wave.speed * TAU * 0.45;
      const fade = Math.min(1, x / (w * 0.12)) * Math.min(1, (w - x) / (w * 0.08));
      const yy = y + g.amp * Math.sin(phase) * fade;
      if (x === 0) ctx.moveTo(x, yy);
      else ctx.lineTo(x, yy);
    }
    ctx.globalAlpha = colors.dark ? 0.55 : 0.5;
    ctx.stroke();
  }
  ctx.globalAlpha = 1;
}

export default function HeroLight() {
  const canvasRef = useRef(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return undefined;
    const ctx = canvas.getContext("2d");
    if (!ctx) return undefined;
    let colors = readColors(canvas);
    let g = geometry(1, 1);
    let frame = 0;
    let visible = false;
    let running = false;
    const started = performance.now();
    const reducedQuery = window.matchMedia("(prefers-reduced-motion: reduce)");

    const paint = () => draw(ctx, g, colors, reducedQuery.matches ? 0 : (performance.now() - started) / 1000);
    const loop = () => {
      paint();
      frame = requestAnimationFrame(loop);
    };
    const sync = () => {
      const should = visible && !document.hidden && !reducedQuery.matches;
      if (should && !running) {
        running = true;
        frame = requestAnimationFrame(loop);
      } else if (!should && running) {
        running = false;
        cancelAnimationFrame(frame);
        paint();
      }
    };
    const resize = () => {
      const rect = canvas.getBoundingClientRect();
      const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
      canvas.width = Math.max(1, Math.round(rect.width * dpr));
      canvas.height = Math.max(1, Math.round(rect.height * dpr));
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      g = geometry(rect.width, rect.height);
      paint();
    };
    resize();

    const resizeObserver = new ResizeObserver(resize);
    resizeObserver.observe(canvas);
    const intersection = new IntersectionObserver((entries) => {
      visible = entries.some((e) => e.isIntersecting);
      sync();
    });
    intersection.observe(canvas);
    const themeObserver = new MutationObserver(() => {
      colors = readColors(canvas);
      paint();
    });
    themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme", "class"] });
    const scheme = window.matchMedia("(prefers-color-scheme: dark)");
    const onScheme = () => {
      colors = readColors(canvas);
      paint();
    };
    scheme.addEventListener("change", onScheme);
    reducedQuery.addEventListener("change", sync);
    document.addEventListener("visibilitychange", sync);

    return () => {
      cancelAnimationFrame(frame);
      resizeObserver.disconnect();
      intersection.disconnect();
      themeObserver.disconnect();
      scheme.removeEventListener("change", onScheme);
      reducedQuery.removeEventListener("change", sync);
      document.removeEventListener("visibilitychange", sync);
    };
  }, []);

  return <canvas ref={canvasRef} aria-hidden className="pointer-events-none mt-12 block h-44 w-full sm:h-52" />;
}
