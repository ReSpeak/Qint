import { defineConfig } from "@rsbuild/core";
import { pluginSvelte } from "@rsbuild/plugin-svelte";
import { pluginSass } from "@rsbuild/plugin-sass";

export default defineConfig({
	plugins: [pluginSvelte(), pluginSass()],
	source: { entry: { index: "./harness/reconnect/main.ts" } },
	server: { port: 4501 },
});
