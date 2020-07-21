import svelte from "rollup-plugin-svelte";
import replace from '@rollup/plugin-replace';
import resolve from "@rollup/plugin-node-resolve";
import commonjs from "@rollup/plugin-commonjs";
import livereload from "rollup-plugin-livereload";
import { terser } from "rollup-plugin-terser";
import copy from 'rollup-plugin-copy';
import typescript from '@rollup/plugin-typescript';

const svelteOptions = require("./svelte.config");

const production = !process.env.ROLLUP_WATCH;

export default {
	input: "src/main.ts",
	output: {
		sourcemap: true,
		format: "iife",
		name: "app",
		file: "public/bundle.js"
	},
	plugins: [
		svelte({
			...svelteOptions,
			// enable run-time checks when not in production
			dev: !production,
			// we'll extract any component CSS out into
			// a separate file — better for performance
			css: css => {
				css.write("public/bundle.css");
			},
			onwarn(warning, onwarn) {
				if (!/A11y:/.test(warning.message)) {
					onwarn(warning);
				}
			},
		}),

		replace({
			__buildEnv__: production ? "production" : "development",
			__buildDat__: `${process.env.npm_package_name} - ${process.env.npm_package_version}`
		}),

		// If you have external dependencies installed from
		// npm, you'll most likely need these plugins. In
		// some cases you'll need additional configuration —
		// consult the documentation for details:
		// https://github.com/rollup/rollup-plugin-commonjs
		resolve({
			browser: true,
			dedupe: importee =>
				importee === "svelte" || importee.startsWith("svelte/")
		}),
		commonjs(),
		typescript(),
		copy({
			targets: [
				{ src: './node_modules/@mdi/font/fonts/*', dest: 'public/fonts' },
				{ src: './node_modules/katex/dist/fonts/*', dest: 'public/fonts' }
			]
		}),

		// Watch the `public` directory and refresh the
		// browser on changes when not in production
		!production && livereload("public"),

		// If we're building for production (npm run build
		// instead of npm run dev), minify
		production && terser()
	],
	watch: {
		clearScreen: false
	}
};
