import { importModule } from "@uupaa/dynamic-import-polyfill";
import { backend } from "./backend/backend";
import { Connection } from "./connection";
import { InMsg } from "./backend/ws";
import { NotificationHandler, TsNotification } from "./notifications";
import debug from "debug";
const log = debug("PLUGIN"),
	error = debug("error:PLUGIN");

export const importFunc = genImportFunc();

export interface IPlugin {
	handleEvent?: (con: Connection, evt: InMsg) => any;
	handleNotification?: NotificationHandler;
	handleTts?: (con: Connection, notification: TsNotification) => void;
}

function genImportFunc() {
	try {
		return new Function("url", `return import(url);`);
	} catch (err) {
		return importModule;
	}
}

export async function loadPlugins(): Promise<IPlugin[]> {
	const plugins: IPlugin[] = [];
	let list: string[];
	try {
		list = await backend.plugin_list();
	} catch (err) {
		error("Failed to load plugins list", err);
		return plugins;
	}
	for (let i = 0; i < list.length; i++) {
		try {
			const mod = await backend.plugin_load(list[i]);
			log("Loaded plugin %s: %o", list[i], mod);
			plugins.push(mod);
		} catch (err) {
			error("Failed to load plugin %s: $j", list[i], err);
		}
	}
	return plugins;
}
