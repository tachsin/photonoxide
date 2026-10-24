import { getExamples } from "./examples";
import { EXAMPLES_PATH, LANGUAGES_DOCS_PATH, METHODS_PATH, PYTHON_DOCS_PATH } from "./meta";
import { getMethods } from "./methods";

/**
 * The docs' guides beside getting started, and every method and example page,
 * for the app's sitemap (the registry's `extraPaths`). The methods and examples
 * are left out when GitHub can't be reached.
 * @returns {Promise<string[]>}
 */
export async function photonoxidePaths() {
  const [{ methods }, { examples }] = await Promise.all([getMethods(), getExamples()]);
  return [
    PYTHON_DOCS_PATH,
    LANGUAGES_DOCS_PATH,
    ...methods.map((m) => `${METHODS_PATH}/${m.slug}`),
    ...examples.map((e) => `${EXAMPLES_PATH}/${e.slug}`),
  ];
}
