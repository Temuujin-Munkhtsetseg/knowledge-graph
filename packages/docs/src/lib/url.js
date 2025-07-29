/**
 * Provides a compatible base url for the docs site to be deployed to GitLab Pages.
 *
 * The `PAGES_PREFIX` environment variable is set by the GitLab Pages to the branch name.
 *
 * Use this when creating any custom components that need to link to other pages.
 */
export const getBaseUrl = () => {
  if (import.meta.env.CI) {
    return import.meta.env.PAGES_PREFIX
      ? `/rust/knowledge-graph/${import.meta.env.PAGES_PREFIX}/`
      : "/rust/knowledge-graph/";
  }
  return "/";
};
