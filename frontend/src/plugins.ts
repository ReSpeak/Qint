import { BASE_ADDRESS } from "./util";
import { importModule } from "@uupaa/dynamic-import-polyfill";
import { backend } from "./backend/backend";
import { app } from "./app";

const importFunc = genImportFunc();

function genImportFunc() {
	try {
		return new Function("url", `return import("${BASE_ADDRESS}/plugins/" + url);`);
	} catch (err) {
		return (url: string) => importModule(`${BASE_ADDRESS}/plugins/${url}`);
	}
}

export async function loadPlugins() {
	app.plugins = [];
	let list: string[];
	try {
		list = await (await backend.fetch(`/plugins`)).json();
	} catch (err) {
		console.log("Failed to load plugins list", err);
		return;
	}
	for (let i = 0; i < list.length; i++) {
		try {
			const mod = await importFunc(list[i]);
			app.plugins.push(mod);
		} catch (err) {
			console.error(`Failed to load plugin ${list[i]}`);
		}
	}
}
