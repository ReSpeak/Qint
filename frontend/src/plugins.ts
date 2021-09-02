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

function getCssPlugin(fileName: string): HTMLStyleElement | null {
	return document.getElementById(`css-${fileName}`) as HTMLStyleElement;
}

export function loadCssPlugin(fileName: string, body: string): void {
	let element = getCssPlugin(fileName);
	if (element == null) {
		element = document.createElement("style");
		element.id = `css-${fileName}`;
		document.head.appendChild(element);
	}
	element.innerHTML = body;
}

export function removeCssPlugin(fileName: string): void {
	log("removing %s", fileName);
	const element = getCssPlugin(fileName);
	if (element != null) {
		element.remove();
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
			if (list[i].endsWith(".js")) {
				const mod = await backend.plugin_load(list[i]);
				log("Loaded js plugin %s: %o", list[i], mod);
				plugins.push(mod);
			} else if (list[i].endsWith(".css")) {
				const cssBody = await backend.plugin_get(list[i]);
				loadCssPlugin(list[i], cssBody);
				log("Loaded css plugin %s", list[i]);
			} else {
				error("Failed to load plugin %s: Unknown file type.", list[i]);
			}
		} catch (err) {
			error("Failed to load plugin %s: $j", list[i], err);
		}
	}
	return plugins;
}
