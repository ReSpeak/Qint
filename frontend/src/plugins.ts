import { BASE_ADDRESS } from "./util";

export let plugins: any[] = [];

export async function loadPlugins() {
	plugins = [];
	let list: string[];
	try {
		list = await (await fetch(`${BASE_ADDRESS}/plugins`)).json();
	} catch (err) {
		console.log("Failed to load plugins list", err);
		return;
	}
	for (let i = 0; i < list.length; i++) {
		try {
			const mod = await import(`${BASE_ADDRESS}/plugins/${list[i]}`);
			plugins.push(mod);
		} catch (err) {
			console.error(`Failed to load plugin ${list[i]}`);
		}
	}
}
