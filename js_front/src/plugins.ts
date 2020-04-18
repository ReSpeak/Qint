
export let plugins: any[] = [];

export async function loadPlugins() {
	plugins = [];
	const list: string[] = await (await fetch("http://localhost:4422/plugins")).json();
	for (let i = 0; i < list.length; i++) {
		try {
			const mod = await import(`http://localhost:4422/plugins/${list[i]}`);
			plugins.push(mod);
		} catch (err) {
			console.error(`Failed to load plugin ${list[i]}`);
		}
	}
}