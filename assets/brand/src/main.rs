//! Writes photonoxide's brand files: the SVGs, then the PNGs rendered from them.
//!
//! Run from this directory: `cargo run --release -- <SpaceGrotesk[wght].ttf>`; README.md says
//! where the font comes from. `--preview <dir>` also renders every SVG to a PNG there, to look at.

use std::f64::consts::PI;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use ttf_parser::{Face, OutlineBuilder, Tag};

// genoxide's palette: the two projects are one family
const DEEP: &str = "#8a2a10";
const RUST: &str = "#b7410e";
const ORANGE: &str = "#d9611c";
const COPPER: &str = "#ee8a32";
const AMBER: &str = "#f7b955";
const CREAM: &str = "#fbe7c6";
const CARD_TOP: &str = "#2b1d17";
const CARD_BOTTOM: &str = "#161010";
/// "photon" on light backgrounds.
const INK: &str = "#231a16";
/// "photon" on dark backgrounds.
const PAPER: &str = "#f6ede5";
/// The taglines on the cards.
const MUTED: &str = "#d8c3ae";

const TAGLINE: &str = "Photonics for Rust: validated and fabrication-ready";
/// The banners' title, for screen readers.
const TITLE: &str = "photonoxide: validated, fabrication-ready photonics for Rust";
const DETAILS: &str =
    "mode solvers · FDFD · FDTD · inverse design · layout and tape-out · a live studio";

/// A number for an SVG: at most two decimals, no trailing zeros.
fn fmt(v: f64) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

// ---- text, as outlines -------------------------------------------------------------------------

/// Turns a glyph's outline into SVG path commands: scaled, flipped (font units are y-up) and
/// moved to the pen's position.
struct Pen {
    d: String,
    scale: f64,
    x: f64,
    y: f64,
}

impl Pen {
    /// A point, to a tenth of a unit: invisible at these sizes, and it halves the files.
    fn point(&self, x: f32, y: f32) -> String {
        let r = |v: f64| fmt((v * 10.0).round() / 10.0);
        format!(
            "{} {}",
            r(self.x + f64::from(x) * self.scale),
            r(self.y - f64::from(y) * self.scale)
        )
    }
}

impl OutlineBuilder for Pen {
    fn move_to(&mut self, x: f32, y: f32) {
        let p = self.point(x, y);
        write!(self.d, "M{p}").unwrap();
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.point(x, y);
        write!(self.d, "L{p}").unwrap();
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (c, p) = (self.point(x1, y1), self.point(x, y));
        write!(self.d, "Q{c} {p}").unwrap();
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (a, b, p) = (self.point(x1, y1), self.point(x2, y2), self.point(x, y));
        write!(self.d, "C{a} {b} {p}").unwrap();
    }
    fn close(&mut self) {
        self.d.push('Z');
    }
}

/// Space Grotesk, the variable font, at any weight.
struct Font(Vec<u8>);

impl Font {
    /// The outline of `text` as one SVG path, its baseline's left end at (x, y); and its width.
    /// `tracking` is extra space between letters, in ems.
    fn text(
        &self,
        text: &str,
        weight: f32,
        size: f64,
        x: f64,
        y: f64,
        tracking: f64,
    ) -> (String, f64) {
        let mut face = Face::parse(&self.0, 0).expect("a TrueType font");
        face.set_variation(Tag::from_bytes(b"wght"), weight)
            .expect("a variable font with a weight axis");
        let scale = size / f64::from(face.units_per_em());
        let mut pen = Pen {
            d: String::new(),
            scale,
            x,
            y,
        };
        for c in text.chars() {
            let id = face
                .glyph_index(c)
                .unwrap_or_else(|| panic!("no glyph for {c:?}"));
            face.outline_glyph(id, &mut pen);
            pen.x += f64::from(face.glyph_hor_advance(id).unwrap_or(0)) * scale + tracking * size;
        }
        let width = pen.x - x - tracking * size;
        (pen.d, width)
    }
}

// ---- the mark ----------------------------------------------------------------------------------

/// A regular hexagon, a vertex up.
fn hexagon(cx: f64, cy: f64, r: f64) -> String {
    let points: Vec<String> = (0..6)
        .map(|k| {
            let a = (-90.0 + 60.0 * f64::from(k)).to_radians();
            format!("{} {}", fmt(cx + r * a.cos()), fmt(cy + r * a.sin()))
        })
        .collect();
    format!("M{} Z", points.join(" L"))
}

/// The mark in a `size` square at (x, y): genoxide's hexagon, holding a silicon strip
/// waveguide in its guided mode, the light brightest in the core and fading into the oxide.
fn mark(p: &str, size: f64, x: f64, y: f64) -> String {
    let s = size / 128.0;
    let rings: String = [(30.0, 19.0, 0.5), (39.0, 25.5, 0.32), (47.0, 31.5, 0.18)]
        .iter()
        .map(|&(rx, ry, opacity)| {
            format!(r#"<ellipse cx="64" cy="64" rx="{rx}" ry="{ry}" opacity="{opacity}"/>"#)
        })
        .collect();
    format!(
        r##"<g transform="translate({x} {y}) scale({s})">
  <defs>
    <linearGradient id="{p}-face" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{CARD_TOP}"/><stop offset="1" stop-color="{CARD_BOTTOM}"/></linearGradient>
    <linearGradient id="{p}-rim" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{COPPER}"/><stop offset="1" stop-color="{DEEP}"/></linearGradient>
    <radialGradient id="{p}-mode" cx="0.5" cy="0.5" r="0.5"><stop offset="0" stop-color="{AMBER}"/><stop offset="0.35" stop-color="{COPPER}" stop-opacity="0.85"/><stop offset="0.7" stop-color="{RUST}" stop-opacity="0.4"/><stop offset="1" stop-color="{RUST}" stop-opacity="0"/></radialGradient>
    <linearGradient id="{p}-core" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="{AMBER}"/><stop offset="0.5" stop-color="{CREAM}"/><stop offset="1" stop-color="{AMBER}"/></linearGradient>
  </defs>
  <path d="{hexagon}" fill="url(#{p}-face)" stroke="url(#{p}-rim)" stroke-width="6" stroke-linejoin="round"/>
  <ellipse cx="64" cy="64" rx="47" ry="32" fill="url(#{p}-mode)"/>
  <g fill="none" stroke="{CREAM}" stroke-width="2.5">{rings}</g>
  <rect x="41" y="55" width="46" height="18" rx="2.5" fill="url(#{p}-core)"/>
</g>"##,
        x = fmt(x),
        y = fmt(y),
        s = fmt(s),
        hexagon = hexagon(64.0, 64.0, 58.0),
    )
}

fn svg(width: f64, height: f64, body: &str, title: &str) -> String {
    let (w, h) = (fmt(width), fmt(height));
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}" role="img" aria-label="{title}">
<title>{title}</title>
{body}
</svg>
"#
    )
}

// ---- the wordmark ------------------------------------------------------------------------------

/// "photonoxide" with "oxide" in the rust gradient, its baseline's left end at (x, y); and its
/// width.
fn wordmark(font: &Font, p: &str, photon_color: &str, size: f64, x: f64, y: f64) -> (String, f64) {
    let tracking = -0.02;
    let (photon, photon_width) = font.text("photon", 700.0, size, x, y, tracking);
    let start = x + photon_width + tracking * size;
    let (oxide, oxide_width) = font.text("oxide", 700.0, size, start, y, tracking);
    let width = photon_width + tracking * size + oxide_width;
    let body = format!(
        r#"<defs><linearGradient id="{p}-oxide" x1="{x1}" y1="0" x2="{x2}" y2="0" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="{RUST}"/><stop offset="0.55" stop-color="{COPPER}"/><stop offset="1" stop-color="{AMBER}"/></linearGradient></defs>
<path d="{photon}" fill="{photon_color}"/>
<path d="{oxide}" fill="url(#{p}-oxide)"/>"#,
        x1 = fmt(start),
        x2 = fmt(x + width),
    );
    (body, width)
}

// ---- light, on the banners ---------------------------------------------------------------------

/// A small deterministic generator (SplitMix64), so the files don't change between runs.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Photons around a beam's axis at (cx, cy), spread along it: denser and brighter near the
/// centre.
fn photons(
    cx: f64,
    cy: f64,
    spread: f64,
    count: usize,
    seed: u64,
    width: f64,
    height: f64,
) -> String {
    let mut rng = Rng(seed);
    let colors = [DEEP, RUST, ORANGE, COPPER, AMBER];
    let mut dots = String::new();
    for _ in 0..count {
        let r = spread * (-2.0 * (1.0 - rng.next() * 0.999).ln()).sqrt();
        let a = rng.next() * 2.0 * PI;
        let (x, y) = (cx + r * a.cos() * 2.4, cy + r * a.sin() * 0.8);
        if !(8.0 < x && x < width - 8.0 && 8.0 < y && y < height - 8.0) {
            continue;
        }
        let closeness = (-(r / spread).powi(2) / 2.0).exp();
        let color = colors[((closeness * 5.0) as usize).min(4)];
        write!(
            dots,
            r#"<circle cx="{}" cy="{}" r="{}" fill="{color}" opacity="{}"/>"#,
            fmt(x),
            fmt(y),
            fmt(1.2 + 2.4 * closeness),
            fmt(0.25 + 0.6 * closeness)
        )
        .unwrap();
    }
    dots
}

/// A guided wave along y = cy, centred at cx: its field under a Gaussian envelope (1/e at
/// ± `envelope`), three snapshots a third of a period apart, brightest first, over the faint band
/// of the waveguide's core; all fade out at ± 2 `envelope`. Its gradients' ids start with `id`.
fn wave(id: &str, cx: f64, cy: f64, amplitude: f64, period: f64, envelope: f64) -> String {
    let (a, b) = (cx - 2.0 * envelope, cx + 2.0 * envelope);
    let fade = |n: &str, color: &str, opacity: f64| {
        format!(
            r#"<linearGradient id="{id}-{n}" x1="{}" y1="0" x2="{}" y2="0" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="{color}" stop-opacity="0"/><stop offset="0.3" stop-color="{color}" stop-opacity="{}"/><stop offset="0.5" stop-color="{color}" stop-opacity="{opacity}"/><stop offset="0.7" stop-color="{color}" stop-opacity="{}"/><stop offset="1" stop-color="{color}" stop-opacity="0"/></linearGradient>"#,
            fmt(a),
            fmt(b),
            fmt(0.6 * opacity),
            fmt(0.6 * opacity)
        )
    };
    let snapshots = [(AMBER, 0.6), (COPPER, 0.38), (RUST, 0.24)];
    let mut defs = fade("band", COPPER, 0.09);
    for (k, &(color, opacity)) in snapshots.iter().enumerate() {
        defs += &fade(&k.to_string(), color, opacity);
    }
    let mut out = format!(
        r#"<defs>{defs}</defs><rect x="{}" y="{}" width="{}" height="18" fill="url(#{id}-band)"/>"#,
        fmt(a),
        fmt(cy - 9.0),
        fmt(b - a)
    );
    for k in 0..snapshots.len() {
        let phase = 2.0 * PI * k as f64 / 3.0;
        let mut points = Vec::new();
        let mut x = a;
        while x <= b {
            let envelope = amplitude * (-((x - cx) / envelope).powi(2)).exp();
            let y = cy + envelope * (2.0 * PI * x / period + phase).sin();
            points.push(format!("{} {}", fmt(x), fmt(y)));
            x += 3.0;
        }
        write!(
            out,
            r#"<path d="M{}" fill="none" stroke="url(#{id}-{k})" stroke-width="2.2" stroke-linecap="round"/>"#,
            points.join(" L")
        )
        .unwrap();
    }
    out
}

/// Faint contours of a mode's intensity around (cx, cy).
fn contours(cx: f64, cy: f64, count: u32, step: f64) -> String {
    let rings: String = (1..=count)
        .map(|k| {
            let (rx, ry) = (f64::from(k) * step * 1.6, f64::from(k) * step);
            format!(
                r#"<ellipse cx="{}" cy="{}" rx="{}" ry="{}"/>"#,
                fmt(cx),
                fmt(cy),
                fmt(rx),
                fmt(ry)
            )
        })
        .collect();
    format!(
        r#"<g fill="none" stroke="{COPPER}" stroke-opacity="0.09" stroke-width="1.2">{rings}</g>"#
    )
}

/// The dark card behind a banner; its ids start with `p`, and `url(#{p}-clip)` clips to it.
fn card(p: &str, width: f64, height: f64, radius: f64, glow_x: f64, glow_y: f64) -> String {
    let (w, h, r) = (fmt(width), fmt(height), fmt(radius));
    format!(
        r#"<defs>
  <linearGradient id="{p}-card" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{CARD_TOP}"/><stop offset="1" stop-color="{CARD_BOTTOM}"/></linearGradient>
  <radialGradient id="{p}-glow" cx="{gx}" cy="{gy}" r="0.6"><stop offset="0" stop-color="{RUST}" stop-opacity="0.35"/><stop offset="1" stop-color="{RUST}" stop-opacity="0"/></radialGradient>
  <clipPath id="{p}-clip"><rect width="{w}" height="{h}" rx="{r}"/></clipPath>
</defs>
<rect width="{w}" height="{h}" rx="{r}" fill="url(#{p}-card)"/>
<rect width="{w}" height="{h}" rx="{r}" fill="url(#{p}-glow)"/>"#,
        gx = fmt(glow_x / width),
        gy = fmt(glow_y / height),
    )
}

// ---- the files ---------------------------------------------------------------------------------

/// A finished SVG: its file name, its width in pixels, and its text.
struct File {
    name: String,
    width: f64,
    text: String,
}

fn files(font: &Font) -> Vec<File> {
    let mut out = Vec::new();
    let mut add = |name: String, width: f64, text: String| out.push(File { name, width, text });

    // every id starts with the file's own prefix: the SVGs can be inlined on one page together
    add(
        "logo.svg".into(),
        128.0,
        svg(
            128.0,
            128.0,
            &mark("px-logo", 128.0, 0.0, 0.0),
            "photonoxide",
        ),
    );

    // the wordmark alone, and the mark beside it, for light and dark backgrounds
    for (theme, color) in [("light", INK), ("dark", PAPER)] {
        let (body, width) = wordmark(
            font,
            &format!("px-wordmark-{theme}"),
            color,
            96.0,
            0.0,
            76.0,
        );
        let w = (width + 4.0).ceil();
        add(
            format!("wordmark-{theme}.svg"),
            w,
            svg(w, 100.0, &body, "photonoxide"),
        );

        let p = format!("px-lockup-{theme}");
        let (body, width) = wordmark(font, &p, color, 96.0, 140.0, 88.0);
        let w = (140.0 + width + 4.0).ceil();
        add(
            format!("lockup-{theme}.svg"),
            w,
            svg(
                w,
                128.0,
                &(mark(&p, 128.0, 0.0, 0.0) + &body),
                "photonoxide",
            ),
        );
    }

    // the banner: a dark card, the same on both themes, 4:1
    let (w, h, p) = (1280.0, 320.0, "px-banner");
    let (_, width) = wordmark(font, p, PAPER, 112.0, 0.0, 0.0);
    let left = (w - (200.0 + 28.0 + width)) / 2.0;
    let (word, _) = wordmark(font, p, PAPER, 112.0, left + 228.0, 170.0);
    let (tagline, tagline_width) = font.text(TAGLINE, 500.0, 30.0, left + 232.0, 226.0, 0.0);
    assert!(
        left > 40.0 && left + 232.0 + tagline_width < w - 40.0,
        "the banner's text doesn't fit"
    );
    let banner = card(p, w, h, 28.0, left + 100.0, 160.0)
        + &format!(r#"<g clip-path="url(#{p}-clip)">"#)
        + &contours(1190.0, 70.0, 8, 22.0)
        + &wave(&format!("{p}-wave-a"), 1190.0, 70.0, 30.0, 56.0, 150.0)
        + &photons(1190.0, 70.0, 40.0, 90, 7, w, h)
        + &wave(&format!("{p}-wave-b"), 120.0, 268.0, 20.0, 44.0, 120.0)
        + &photons(120.0, 268.0, 34.0, 45, 11, w, h)
        + "</g>"
        + &mark(p, 200.0, left, 60.0)
        + &word
        + &format!(r#"<path d="{tagline}" fill="{MUTED}"/>"#);
    add("banner.svg".into(), w, svg(w, h, &banner, TITLE));

    // the social preview, 2:1 (rendered to PNG for GitHub)
    let (w, h, p) = (1280.0, 640.0, "px-social");
    let (_, width) = wordmark(font, p, PAPER, 132.0, 0.0, 0.0);
    let left = (w - (236.0 + 32.0 + width)) / 2.0;
    let (word, _) = wordmark(font, p, PAPER, 132.0, left + 268.0, 285.0);
    let (_, tagline_width) = font.text(TAGLINE, 500.0, 36.0, 0.0, 0.0, 0.0);
    let (tagline, _) = font.text(TAGLINE, 500.0, 36.0, (w - tagline_width) / 2.0, 440.0, 0.0);
    let (_, details_width) = font.text(DETAILS, 400.0, 22.0, 0.0, 0.0, 0.0);
    let (details, _) = font.text(DETAILS, 400.0, 22.0, (w - details_width) / 2.0, 490.0, 0.0);
    assert!(
        left > 40.0 && details_width < w - 80.0,
        "the social preview's text doesn't fit"
    );
    let social = card(p, w, h, 0.0, left + 118.0, 260.0)
        + &format!(r#"<g clip-path="url(#{p}-clip)">"#)
        + &contours(1130.0, 110.0, 9, 26.0)
        + &wave(&format!("{p}-wave-a"), 1130.0, 110.0, 36.0, 64.0, 190.0)
        + &photons(1130.0, 110.0, 52.0, 140, 3, w, h)
        + &wave(&format!("{p}-wave-b"), 120.0, 590.0, 22.0, 48.0, 130.0)
        + &photons(120.0, 590.0, 36.0, 55, 5, w, h)
        + "</g>"
        + &mark(p, 236.0, left, 145.0)
        + &word
        + &format!(r#"<path d="{tagline}" fill="{MUTED}"/>"#)
        + &format!(r#"<path d="{details}" fill="{MUTED}" opacity="0.7"/>"#);
    add("social-preview.svg".into(), w, svg(w, h, &social, TITLE));
    out
}

/// Renders an SVG to a PNG `width` pixels wide.
fn png(svg: &str, width: u32, path: &Path) {
    let tree =
        resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).expect("a valid SVG");
    let size = tree.size();
    let scale = width as f32 / size.width();
    let height = (size.height() * scale).round() as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).expect("a non-empty image");
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.save_png(path).expect("a writable file");
    println!(
        "{}: {width} x {height}",
        path.file_name().unwrap().to_string_lossy()
    );
}

fn main() {
    let mut args = std::env::args().skip(1);
    let font_path = args
        .next()
        .expect("usage: photonoxide-brand <SpaceGrotesk[wght].ttf> [--preview <dir>]");
    let preview: Option<PathBuf> = match (args.next().as_deref(), args.next()) {
        (Some("--preview"), Some(dir)) => Some(dir.into()),
        (None, _) => None,
        _ => panic!("usage: photonoxide-brand <SpaceGrotesk[wght].ttf> [--preview <dir>]"),
    };
    let font = Font(std::fs::read(&font_path).expect("a readable font file"));
    let out = Path::new(env!("CARGO_MANIFEST_DIR"));

    let files = files(&font);
    for f in &files {
        std::fs::write(out.join(&f.name), &f.text).expect("a writable file");
        println!("{}: {:.1} KB", f.name, f.text.len() as f64 / 1024.0);
        if let Some(dir) = &preview {
            png(
                &f.text,
                f.width as u32,
                &dir.join(f.name.replace(".svg", ".png")),
            );
        }
    }

    // the icons and the social preview, as PNGs
    let logo = &files.iter().find(|f| f.name == "logo.svg").unwrap().text;
    for size in [16, 32, 180, 192, 512] {
        png(logo, size, &out.join(format!("logo-{size}.png")));
    }
    let social = &files
        .iter()
        .find(|f| f.name == "social-preview.svg")
        .unwrap()
        .text;
    png(social, 1280, &out.join("social-preview.png"));
}
