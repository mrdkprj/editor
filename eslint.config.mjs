import { defineConfig, globalIgnores } from "eslint/config";
import typescriptEslint from "@typescript-eslint/eslint-plugin";
import globals from "globals";
import tsParser from "@typescript-eslint/parser";
import parser from "svelte-eslint-parser";
import path from "node:path";
import { fileURLToPath } from "node:url";
import js from "@eslint/js";
import { FlatCompat } from "@eslint/eslintrc";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const compat = new FlatCompat({
    baseDirectory: __dirname,
    recommendedConfig: js.configs.recommended,
    allConfig: js.configs.all,
});

export default defineConfig([
    globalIgnores([
        "**/logs",
        "**/*.log",
        "**/npm-debug.log*",
        "**/yarn-debug.log*",
        "**/yarn-error.log*",
        "**/lerna-debug.log*",
        "**/pids",
        "**/*.pid",
        "**/*.seed",
        "**/*.pid.lock",
        "**/.DS_Store",
        "**/node_modules/",
        "**/jspm_packages/",
        "**/out/",
        "**/resources",
        "!resources/.keep",
        "**/temp/",
        "**/dist/",
        "**/.vite/",
        "**/package/",
        "**/electron.vite.config*.mjs",
    ]),
    {
        extends: compat.extends("eslint:recommended", "plugin:@typescript-eslint/eslint-recommended", "plugin:@typescript-eslint/recommended", "plugin:svelte/recommended"),
        plugins: {
            "@typescript-eslint": typescriptEslint,
        },
        languageOptions: {
            globals: {
                ...Object.fromEntries(Object.entries(globals.browser).map(([key]) => [key, "off"])),
                ...globals.node,
            },
            parser: tsParser,
            ecmaVersion: "latest",
            sourceType: "module",
            parserOptions: {
                extraFileExtensions: [".svelte"],
            },
        },

        rules: {
            "@typescript-eslint/no-explicit-any": "off",
            "@typescript-eslint/no-var-requires": "off",
            "no-unused-vars": "off",
            "@typescript-eslint/no-unused-vars": [
                "error",
                {
                    argsIgnorePattern: "^_",
                },
            ],
            "no-empty-function": "off",
            "@typescript-eslint/no-empty-function": [
                "error",
                {
                    allow: ["asyncMethods"],
                },
            ],
            "no-cond-assign": "error",
            "@typescript-eslint/explicit-function-return-type": "off",
        },
    },
    {
        files: ["**/*.svelte"],
        languageOptions: {
            parser: parser,
            ecmaVersion: 5,
            sourceType: "script",
            parserOptions: {
                parser: "@typescript-eslint/parser",
            },
        },
    },
]);
