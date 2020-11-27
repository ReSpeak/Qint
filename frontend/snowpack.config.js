let production = false;

module.exports = {
	mount: {
		public: '/',
		src: '/_dist_',
	},
	plugins: [
		'@snowpack/plugin-svelte',
		'@snowpack/plugin-dotenv',
		"@snowpack/plugin-typescript",
		[
			'snowpack-plugin-replace',
			{
				list: [
					{
						from: '__buildEnv__',
						to: production ? "production" : "development"
					},
					{
						from: '__buildDat__',
						to: `${process.env.npm_package_name} - ${process.env.npm_package_version}`
					}
				],
			},
		],
		//"@snowpack/plugin-optimize",
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
