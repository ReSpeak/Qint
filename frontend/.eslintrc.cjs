module.exports = {
	root: true,
	parser: "@typescript-eslint/parser",
	extends: ["eslint:recommended", "plugin:@typescript-eslint/recommended", "prettier"],
	plugins: ["svelte3", "@typescript-eslint"],
	ignorePatterns: ["*.cjs", "vite.config.js"],
	rules: {
		"@typescript-eslint/no-explicit-any": "off",
		"@typescript-eslint/no-non-null-assertion": "off",
		"@typescript-eslint/no-inferrable-types": "off",
		"@typescript-eslint/no-unused-vars": ["warn", { argsIgnorePattern: "^_" }],
		"no-debugger": "off",
		"no-empty": "off",
		"@typescript-eslint/no-empty-function": "off",
		"prefer-const": ["error", { destructuring: "all" }],
	},
	overrides: [{ files: ["*.svelte"], processor: "svelte3/svelte3" }],
	settings: {
		"svelte3/typescript": require("typescript"),
		"svelte3/ignore-styles": ({ lang }) => lang === "scss",
		"svelte3/ignore-warnings": ({ code }) => code.includes('a11y'),
	},
	parserOptions: {
		sourceType: "module",
		ecmaVersion: 2020,
	},
	env: {
		browser: true,
		es2017: true,
	}
};
