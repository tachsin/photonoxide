// Labels beside a picture, kept from overlapping: a layer a few nanometres thick has its name
// centred on a band thinner than a line of text, so it would sit on its neighbours' names. Plain
// functions with no imports, so Node's test runner takes them as they are (studio/tests/labels.test.ts).

/**
 * The heights to draw labels at, each as near its wanted height `wanted[i]` as it can be with
 * every two at least `gap` apart and all within `[lo, hi]` (screen heights, down the page): the
 * labels keep their order, and where some crowd together they spread out evenly about the middle
 * of the heights they wanted. Where there is no room for them all, they are spaced `gap` apart
 * from `lo`, and the last ones run past `hi`.
 */
export function spread(wanted: number[], gap: number, lo: number, hi: number): number[] {
  const order = wanted.map((y, i) => [y, i] as const).sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  // clusters of labels that would overlap, each placed about its members' mean
  type Cluster = { first: number; count: number; sum: number; start: number };
  const clusters: Cluster[] = [];
  const place = (c: Cluster) => {
    const span = (c.count - 1) * gap;
    c.start = Math.min(Math.max(c.sum / c.count - span / 2, lo), hi - span);
  };
  for (let k = 0; k < order.length; k++) {
    const c: Cluster = { first: k, count: 1, sum: order[k][0], start: 0 };
    place(c);
    clusters.push(c);
    // merge with the cluster above while they overlap
    while (clusters.length > 1) {
      const [a, b] = [clusters[clusters.length - 2], clusters[clusters.length - 1]];
      if (a.start + a.count * gap <= b.start + 1e-9) break;
      a.count += b.count;
      a.sum += b.sum;
      place(a);
      clusters.pop();
    }
  }
  const out = new Array<number>(wanted.length);
  for (const c of clusters) {
    const start = Math.max(c.start, lo);
    for (let j = 0; j < c.count; j++) out[order[c.first + j][1]] = start + j * gap;
  }
  return out;
}
