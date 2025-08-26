import { visit } from "unist-util-visit";

/**
 * Remark plugin to add base path to absolute markdown links
 * This is needed because Astro doesn't handle base paths for markdown links
 * See: https://github.com/withastro/starlight/discussions/1763
 */
export function remarkBaseUrl() {
  return function transformer(tree, file) {
    // Determine base path based on environment
    let basePath = "/";

    if (process.env.CI) {
      const baseProject = "/rust/knowledge-graph";
      if (process.env.PAGES_PREFIX) {
        basePath = `${baseProject}/${process.env.PAGES_PREFIX}/`;
      } else {
        basePath = `${baseProject}/`;
      }
    }

    // Only process if we have a non-root base path
    if (basePath === "/") {
      return;
    }

    visit(tree, "link", (node) => {
      // Only process internal absolute links
      if (
        node.url &&
        node.url.startsWith("/") &&
        !node.url.startsWith("http")
      ) {
        // Remove trailing slash from base, keep leading slash on URL
        const cleanBase = basePath.endsWith("/")
          ? basePath.slice(0, -1)
          : basePath;
        node.url = cleanBase + node.url;
      }
    });
  };
}

