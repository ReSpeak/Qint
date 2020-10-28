const sveltePreprocess = require('svelte-preprocess');
const plainSass = require('sass');
//const nodeSass = require('node-sass');

module.exports = {
	preprocess: sveltePreprocess({
		defaults: {
			script: 'typescript',
		},
		typescript: {
			tsconfigFile: "tsconfig.json"
		},
		scss: {
			implementation: plainSass,
			includePaths: ['src', 'node_modules'],
			// prependData is for preproc >= 4.X
			prependData: `
			@import "bulmaswatch/cyborg/variables";
			@import "bulma/sass/utilities/all";
			`,
		},
	}),

	// we'll extract any component CSS out into
	// a separate file — better for performance
	css: css => {
		css.write("bundle.css");
	},
	onwarn(warning, onwarn) {
		if (!/A11y:/.test(warning.message)) {
			onwarn(warning);
		}
	},
};
