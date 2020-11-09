import svelte from "rollup-plugin-svelte";
import replace from '@rollup/plugin-replace';
import resolve from "@rollup/plugin-node-resolve";
import commonjs from "@rollup/plugin-commonjs";
import { terser } from "rollup-plugin-terser";
import copy from 'rollup-plugin-copy';
import typescript from '@rollup/plugin-typescript';
import babel from "@rollup/plugin-babel";

const svelteOptions = require("./svelte.config");

//console.log(process.env);
const production = false;

export default {
	input: "src/main.ts",
	output: {
		sourcemap: true,
		format: "umd",
		name: "app",
		file: "public/bundle.js"
	},
	plugins: [
		svelte({
			...svelteOptions,
			// enable run-time checks when not in production
			dev: !production,
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
		babel({
			extensions: ['.js', '.mjs', '.html', '.svelte'],
			include: ['src/**', 'node_modules/svelte/**'],
			babelHelpers: 'bundled'
		}),
		typeCheck(),
		typescript({ sourceMap: !production }),
		copy({
			targets: [
				{ src: './node_modules/@mdi/font/fonts/*', dest: 'public/fonts' },
				{ src: './node_modules/katex/dist/fonts/*', dest: 'public/fonts' }
			]
		}),

		// If we're building for production (npm run build
		// instead of npm run dev), minify
		production && terser()
	],
	watch: {
		clearScreen: false
	}
};

function typeCheck() {
	return {
		writeBundle() {
			require('child_process').spawn('svelte-check', {
				stdio: ['ignore', 'inherit', 'inherit'],
				shell: true
			});
		}
	}
}
