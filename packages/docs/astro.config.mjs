// @ts-check
import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import tailwindcss from "@tailwindcss/vite";
import {
  copyFileSync,
  mkdirSync,
  writeFileSync,
  readdirSync,
  existsSync,
} from "fs";
import { join } from "path";

function copyGitLabFonts() {
  let fontsCopied = false;

  const copyFonts = () => {
    if (fontsCopied) return;

    try {
      const fontsSource = "../../node_modules/@gitlab/fonts";
      const fontsDest = "src/fonts";

      if (!existsSync(fontsSource)) {
        console.warn("GitLab fonts not found, skipping...");
        return;
      }

      mkdirSync(fontsDest, { recursive: true });

      /** @type {string[]} */
      const fontFaces = [];
      const families = ["gitlab-sans", "gitlab-mono", "jetbrains-mono"];

      families.forEach((family) => {
        const sourceDir = join(fontsSource, family);
        const destDir = join(fontsDest, family);

        if (existsSync(sourceDir)) {
          mkdirSync(destDir, { recursive: true });

          readdirSync(sourceDir)
            .filter((file) => file.endsWith(".woff2"))
            .forEach((file) => {
              copyFileSync(join(sourceDir, file), join(destDir, file));
              console.log(`✓ Copied ${family}/${file}`);

              const fontName = family
                .split("-")
                .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
                .join(" ");
              const weight = file.includes("Bold") ? "bold" : "normal";
              const style = file.includes("Italic") ? "italic" : "normal";

              fontFaces.push(`@font-face {
  font-family: '${fontName}';
  src: url('./${family}/${file}') format('woff2');
  font-weight: ${weight};
  font-style: ${style};
  font-display: swap;
}`);
            });
        }
      });

      const css = `/* GitLab Fonts */
${fontFaces.join("\n\n")}

:root {
  --font-gitlab-sans: 'Gitlab Sans', system-ui, sans-serif;
  --font-gitlab-mono: 'Gitlab Mono', 'JetBrains Mono', monospace;
  --font-jetbrains-mono: 'Jetbrains Mono', monospace;
}`;

      writeFileSync(join(fontsDest, "font-face.css"), css);
      console.log("✅ Generated font-face.css");
      fontsCopied = true;
    } catch (error) {
      console.warn(
        "Font copy failed:",
        error instanceof Error ? error.message : String(error),
      );
    }
  };

  return {
    name: "copy-gitlab-fonts",
    buildStart: copyFonts,
    configureServer: copyFonts,
  };
}

// https://astro.build/config
// https://astro.build/config
export default defineConfig({
  site: process.env.CI ? "https://gitlab-org.gitlab.io/" : undefined,
  base: process.env.CI
    ? process.env.PAGES_PREFIX
      ? `/rust/knowledge-graph/${process.env.PAGES_PREFIX}/`
      : "/rust/knowledge-graph/"
    : "/",
  trailingSlash: "always",
  build: {
    format: "directory",
  },
  integrations: [
    starlight({
      title: "GitLab Knowledge Graph",
      logo: {
        light: "./src/assets/gkg-logo.png",
        dark: "./src/assets/gkg-logo.png",
      },
      favicon: "favicon.ico",
      components: {
        SiteTitle: "./src/components/SiteTitle.astro",
      },
      customCss: ["./src/fonts/font-face.css", "./src/styles/global.css"],
      social: [
        {
          icon: "gitlab",
          label: "GitLab",
          href: "https://gitlab.com/gitlab-org/rust/knowledge-graph",
        },
      ],
      sidebar: [
        {
          label: "Getting Started",
          autogenerate: { directory: "getting-started" },
        },
        {
          label: "CLI Reference",
          autogenerate: { directory: "cli" },
        },
        {
          label: "Architecture",
          autogenerate: { directory: "architecture" },
        },
      ],
      editLink: {
        baseUrl:
          "https://gitlab.com/gitlab-org/rust/knowledge-graph/-/edit/main/packages/docs/src/content",
      },
    }),
  ],

  vite: {
    // @ts-ignore - Tailwind CSS v4 plugin compatibility issue with Vite types
    plugins: [tailwindcss(), copyGitLabFonts()],
  },
});
