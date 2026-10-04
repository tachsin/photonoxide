// The themes Settings offers: the studio's own two, then the further ones app.css builds in.

export interface Theme {
  /** The theme's name: its `data-theme`. */
  name: string;
  /** What the settings store for it: its name, but "daisyui-light" and "daisyui-dark" (kept, so
   *  saved settings still read) for the further light and dark, since "light" and "dark" there
   *  mean the studio's own themes. */
  setting: string;
  /** Its color-scheme is dark. */
  dark: boolean;
}

const LIST: [string, boolean][] = [
  ["light", false],
  ["dark", true],
  ["cupcake", false],
  ["bumblebee", false],
  ["emerald", false],
  ["corporate", false],
  ["synthwave", true],
  ["retro", false],
  ["cyberpunk", false],
  ["valentine", false],
  ["halloween", true],
  ["garden", false],
  ["forest", true],
  ["aqua", true],
  ["lofi", false],
  ["pastel", false],
  ["fantasy", false],
  ["wireframe", false],
  ["black", true],
  ["luxury", true],
  ["dracula", true],
  ["cmyk", false],
  ["autumn", false],
  ["business", true],
  ["acid", false],
  ["lemonade", false],
  ["night", true],
  ["coffee", true],
  ["winter", false],
  ["dim", true],
  ["nord", false],
  ["sunset", true],
  ["caramellatte", false],
  ["abyss", true],
  ["silk", false],
];

/** The further themes, in app.css's order. */
export const MORE_THEMES: readonly Theme[] = LIST.map(([name, dark]) => ({
  name,
  setting: name === "light" || name === "dark" ? `daisyui-${name}` : name,
  dark,
}));

/**
 * The `data-theme` a setting stands for. "system", "dark" and "light" are the studio's own themes
 * (by the OS's preference, or chosen); any other value is one of the further themes' (see `Theme.setting`).
 * An unknown one is taken as "system".
 */
export function themeName(setting: string, systemDark: boolean): string {
  if (setting === "dark") return "photonoxide-dark";
  if (setting === "light") return "photonoxide-light";
  const other = MORE_THEMES.find((t) => t.setting === setting);
  if (other) return other.name;
  return systemDark ? "photonoxide-dark" : "photonoxide-light";
}
