import { getExamples } from "./examples";
import { EXAMPLES_PATH, METHODS_PATH } from "./meta";
import { getMethods } from "./methods";

/**
 * Every method and example page, for the app's sitemap (the registry's
 * `extraPaths`). Empty when GitHub can't be reached.
 * @returns {Promise<string[]>}
 */
export async function photonoxidePaths() {
  const [{ methods }, { examples }] = await Promise.all([getMethods(), getExamples()]);
  return [...methods.map((m) => `${METHODS_PATH}/${m.slug}`), ...examples.map((e) => `${EXAMPLES_PATH}/${e.slug}`)];
}
