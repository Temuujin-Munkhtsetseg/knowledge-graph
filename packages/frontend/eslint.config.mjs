import vue from "eslint-plugin-vue";
import typescriptEslint from "@typescript-eslint/eslint-plugin";
import globals from "globals";
import parser from "vue-eslint-parser";
import tsParser from "@typescript-eslint/parser";
import path from "node:path";
import { fileURLToPath } from "node:url";
import js from "@eslint/js";
import { FlatCompat } from "@eslint/eslintrc";
import { importX } from 'eslint-plugin-import-x';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const compat = new FlatCompat({
  baseDirectory: __dirname,
  recommendedConfig: js.configs.recommended,
  allConfig: js.configs.all
});

export default [
  {
    ignores: [
      "**/dist/",
      "**/dist-ssr/",
      "**/coverage/",
      "**/node_modules/",
      "**/*.d.ts",
      "**/vite.config.ts",
      "**/postcss.config.mjs",
      "eslint.config.mjs",
      "**/components/ui/**",
    ]
  },
  js.configs.recommended,
  ...compat.extends("plugin:@gitlab/typescript", "prettier"),
  ...vue.configs["flat/recommended"],
  importX.flatConfigs.recommended,
  importX.flatConfigs.typescript,
  {
    files: ["**/*.ts", "**/*.vue"],
    plugins: {
      "@typescript-eslint": typescriptEslint,
      "import-x": importX,
    },
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
      ecmaVersion: 2022,
      sourceType: "module",
      parser: tsParser,
      parserOptions: {
        project: ["./tsconfig.*.json"],
        extraFileExtensions: [".vue"],
      },
    },
    rules: {
      "@typescript-eslint/array-type": ["error", { default: "array" }],
      "@typescript-eslint/explicit-member-accessibility": ["error", { accessibility: "no-public" }],
      "@typescript-eslint/parameter-properties": ["error", { prefer: "class-property" }],
      "@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_" }],
      "unicorn/filename-case": "off",
      "import/no-unresolved": "off",
      "import/extensions": "off",
      "import/no-cycle": "off",
      "import-x/no-unresolved": "error",
      "import-x/extensions": ["error", "ignorePackages", {
        "js": "never",
        "jsx": "never",
        "ts": "never",
        "tsx": "never",
        "vue": "never"
      }],
    },
    settings: {
      "import-x/parsers": {
        "@typescript-eslint/parser": [".ts", ".tsx"]
      },
      "import-x/resolver": {
        "typescript": {
          "alwaysTryTypes": true,
          "project": ["./tsconfig.json"]
        }
      }
    }
  },
  {
    files: ["**/*.test.ts", "**/test_examples/**/*"],
    rules: {
      "func-names": "off",
      "no-empty-function": "off",
    },
  },
  {
    files: ["**/*.ts"],
    rules: {},
  },
  {
    files: ["**/*.vue"],
    plugins: {
      vue: vue,
      "@typescript-eslint": typescriptEslint,
    },
    languageOptions: {
      parser: parser,
      parserOptions: {
        parser: tsParser,
        project: ["./tsconfig.*.json"],
        extraFileExtensions: [".vue"],
        ecmaFeatures: {
          jsx: true
        }
      },
    },
            rules: {
            "vue/multi-word-component-names": "off",
            "unicorn/filename-case": "off",
            // TODO: fix this
            "vue/valid-v-for": "off",
            // Disable Vue formatting rules that conflict with Prettier
            "vue/max-attributes-per-line": "off",
            "vue/singleline-html-element-content-newline": "off",
            "vue/multiline-html-element-content-newline": "off",
            "vue/html-indent": "off",
            "vue/html-closing-bracket-newline": "off",
      // TODO: fix this maybe
      // Let TypeScript compiler handle unused variables for Vue files
      // ESLint's unused variable detection doesn't understand Vue's <script setup> template usage
      // TypeScript (with noUnusedLocals: true) correctly detects unused variables in Vue files
      // Run `npm run build` or `npx vue-tsc --noEmit` to check for unused variables
      "no-unused-vars": "off",
      "@typescript-eslint/no-unused-vars": "off",
    },
  },
];