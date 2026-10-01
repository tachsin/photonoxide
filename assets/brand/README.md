# photonoxide's brand files

The design is photons + oxide, in genoxide's family: genoxide's hexagon and iron-oxide colors, holding a silicon strip waveguide in its guided mode. The light is brightest in the core and fades into the oxide. The wordmark sets "oxide" in the same gradient, as genoxide's does. The banners carry a guided wave under a Gaussian envelope, with photons around it. It's rust-themed without the Rust logo, a trademark of the Rust Foundation.

| File | What | Where it goes |
|---|---|---|
| `logo.svg` | The mark, 128 × 128 | docs.rs (`html_logo_url` in `src/lib.rs`) |
| `logo-16.png`, `logo-32.png` | Favicons | docs.rs (`html_favicon_url`) |
| `logo-180.png`, `logo-192.png`, `logo-512.png` | App and touch icons | Pages that want them (Apple touch icon 180, Android 192 and 512) |
| `wordmark-light.svg`, `wordmark-dark.svg` | "photonoxide", for light and dark backgrounds | Wherever the name stands alone |
| `lockup-light.svg`, `lockup-dark.svg` | The mark beside the wordmark, for light and dark backgrounds | Headers, slides |
| `banner.svg` | The banner, 1280 × 320: a dark card that suits both themes | The top of README.md |
| `social-preview.svg`, `social-preview.png` | 1280 × 640 | GitHub's social preview (Settings → General → Social preview: upload the PNG), link previews |

README.md and docs.rs load them from `main` on raw.githubusercontent.com, so they aren't in the published crate (`exclude` in the root `Cargo.toml`).

## Making them

Every file comes from the generator in this directory, a small Rust program of its own (not part of photonoxide, never published). Don't edit the files by hand.

```sh
curl -L -o SpaceGrotesk.ttf "https://github.com/google/fonts/raw/main/ofl/spacegrotesk/SpaceGrotesk%5Bwght%5D.ttf"
cd assets/brand
cargo run --release -- ../../SpaceGrotesk.ttf                     # the SVGs and PNGs, here
cargo run --release -- ../../SpaceGrotesk.ttf --preview <dir>     # also a PNG of every SVG, to look at
```

It outlines the text with `ttf-parser`, so the SVGs need no font, and renders the PNGs with `resvg`. The output is the same on every run: the photons come from a seeded generator.

## Colors

genoxide's palette, so the two read as one family:

| Name | Hex | Use |
|---|---|---|
| Deep oxide | `#8a2a10` | The hexagon's rim, the dimmest photons |
| Rust | `#b7410e` | The mode's edge, the start of "oxide" |
| Orange | `#d9611c` | Photons |
| Copper | `#ee8a32` | The rim, the mode, "oxide" |
| Amber | `#f7b955` | The mode's peak, the core's edges, the end of "oxide" |
| Cream | `#fbe7c6` | The core's centre, the mode's contours |
| Card | `#2b1d17` → `#161010` | The hexagon and the banners' background |
| Ink / paper | `#231a16` / `#f6ede5` | "photon" on light / dark backgrounds |

## The font

The wordmark and the banners' text are Space Grotesk (Florian Karsten), under the SIL Open Font License 1.1, as in genoxide. The generator converts the text to outlines, so the font isn't redistributed here; download it from Google Fonts' repository (above).
