const sveltePreprocess = require('svelte-preprocess');

module.exports = {
	// Consult https://github.com/sveltejs/svelte-preprocess
	// for more information about preprocessors
	preprocess: sveltePreprocess({
		scss: {
			includePaths: ['src', 'node_modules'],
			// prependData is for preproc >= 4.X
			prependData: `
				@import "bulmaswatch/cyborg/_variables";
				@import "bulma/sass/utilities/_all";
			`,
		},
	}),

	/*onwarn(warning, onwarn) {
		if (!/A11y:/.test(warning.message)) {
			onwarn(warning);
		}
	},*/
};
