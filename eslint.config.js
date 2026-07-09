import js from "@eslint/js";
import tseslint from "typescript-eslint";
import pluginVue from "eslint-plugin-vue";
import vueParser from "vue-eslint-parser";

export default [
  {
    ignores: ["dist", "src-tauri/**"],
  },
  // DOM 类型 (tsconfig 已含 DOM lib,但 ESLint 的 no-undef 仍需显式声明)
  {
    languageOptions: {
      globals: {
        File: "readonly",
        FileList: "readonly",
        DragEvent: "readonly",
        Event: "readonly",
        HTMLInputElement: "readonly",
        KeyboardEvent: "readonly",
        localStorage: "readonly",
        Blob: "readonly",
        URL: "readonly",
        document: "readonly",
        window: "readonly",
      },
    },
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...pluginVue.configs["flat/recommended"],
  {
    files: ["**/*.vue"],
    languageOptions: {
      parser: vueParser,
      parserOptions: {
        parser: tseslint.parser,
        extraFileExtensions: [".vue"],
      },
    },
  },
  {
    rules: {
      "vue/multi-word-component-names": "off",
      // 以下规则与 Prettier 格式化冲突,交由 Prettier 统一管理:
      "vue/max-attributes-per-line": "off",
      "vue/singleline-html-element-content-newline": "off",
      "vue/html-self-closing": "off",
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_", caughtErrorsIgnorePattern: "^_" },
      ],
    },
  },
];
