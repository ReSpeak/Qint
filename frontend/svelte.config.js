import sveltePreprocess from "svelte-preprocess";

export default {
	preprocess: sveltePreprocess({
		scss: {
			includePaths: ["src", "node_modules"],
			// prependData is for preproc >= 4.X
			prependData: `
				@use "sass:math";
				@import "bulmaswatch/cyborg/_variables";
				@import "bulma/sass/utilities/_all";
			`,
		},
	}),
};
