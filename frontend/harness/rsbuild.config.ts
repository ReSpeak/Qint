// Standalone debug harness: `npx rsbuild dev -c harness/rsbuild.config.ts`
import { defineConfig } from "@rsbuild/core";
import { pluginSvelte } from "@rsbuild/plugin-svelte";
import { pluginSass } from "@rsbuild/plugin-sass";

export default defineConfig({
	plugins: [pluginSvelte(), pluginSass()],
	source: { entry: { index: "./harness/main.ts" } },
	server: { port: 4500 },
});
