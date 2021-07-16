import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// https://vitejs.dev/config/
export default defineConfig(({ command, mode }) => {
	const production = mode === "production" && command !== "serve";
	return {
		define: {
			BUILD_ENV: production ? '"production"' : '"development"',
			BUILD_DAT: `"${process.env.npm_package_name} - ${process.env.npm_package_version}"`,
		},
		plugins: [svelte()],
		json: {
			stringify: true,
		},
		build: {
			minify: false, // easier to debug in production
			brotliSize: false,
		},
	};
});
