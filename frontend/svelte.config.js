const sveltePreprocess = require('svelte-preprocess');

module.exports = {
	preprocess: sveltePreprocess({
		typescript: {
			tsconfigFile: "tsconfig.json"
		},
		scss: {
			includePaths: ['src', 'node_modules'],
			data: `
			@import "bulmaswatch/cyborg/variables";
			@import "bulma/sass/utilities/all";
			`
		},
	}),
	// ...other svelte options
};
