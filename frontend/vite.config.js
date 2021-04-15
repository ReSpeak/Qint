import { defineConfig } from 'vite'
import svelte from '@sveltejs/vite-plugin-svelte'

let production = false;

// https://vitejs.dev/config/
export default defineConfig({
	define: {
		"BUILD_ENV": production ? "\"production\"" : "\"development\"",
		"BUILD_DAT": `"${process.env.npm_package_name} - ${process.env.npm_package_version}"`,
	},
	plugins: [svelte()],
})
