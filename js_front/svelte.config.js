const sveltePreprocess = require('svelte-preprocess');

module.exports = {
	preprocess: sveltePreprocess({
		scss: {
			includePaths: ['src', 'node_modules'],
			data: `
			@import 'bulmaswatch/cyborg/variables';
			@import 'bulma/bulma';
			@import 'bulmaswatch/cyborg/overrides';`
		},
	}),
	// ...other svelte options
};
