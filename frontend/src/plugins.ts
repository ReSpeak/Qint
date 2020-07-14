import { BASE_ADDRESS } from "./util";

export let plugins: any[] = [];

export async function loadPlugins() {
	plugins = [];
	const list: string[] = await (await fetch(`${BASE_ADDRESS}/plugins`)).json();
	for (let i = 0; i < list.length; i++) {
		try {
			const mod = await import(`${BASE_ADDRESS}/plugins/${list[i]}`);
			plugins.push(mod);
		} catch (err) {
			console.error(`Failed to load plugin ${list[i]}`);
		}
	}
}