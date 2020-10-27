module.exports = {
	mount: {
		public: '/',
		src: '/_dist_',
	},
	plugins: [
		'@snowpack/plugin-svelte',
		'@snowpack/plugin-dotenv',
		"@snowpack/plugin-typescript",
		// svelte-check disabled for now since when it reports back errors
		// snowpack will not generate build files.
		// [
		// 	'@snowpack/plugin-run-script', {
		// 		cmd: 'svelte-check --output human',
		// 		watch: '$1 --watch',
		// 		output: 'stream'
		// 	},
		// ]
	]
};
