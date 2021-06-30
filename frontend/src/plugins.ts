import { BASE_ADDRESS } from "./util";
import { importModule } from "@uupaa/dynamic-import-polyfill";
import { backend } from "./backend/backend";
import { Connection } from "./connection";
import { InMsg } from "./backend/ws";
import { NotificationHandler, TsNotification } from "./notificationss";

const importFunc = genImportFunc();

export interface IPlugin {
	handleEvent?: (con: Connection, evt: InMsg) => any;
	handleNotification?: NotificationHandler;
	handleTts?: (con: Connection, notification: TsNotification) => void;
}

function genImportFunc() {
	try {
		return new Function("url", `return import("${BASE_ADDRESS}/plugins/" + url);`);
	} catch (err) {
		return (url: string) => importModule(`${BASE_ADDRESS}/plugins/${url}`);
	}
}

export async function loadPlugins(): Promise<IPlugin[]> {
	const plugins: IPlugin[] = [];
	let list: string[];
	try {
		list = await (await backend.fetch(`/plugins`)).json();
	} catch (err) {
		console.log("Failed to load plugins list", err);
		return plugins;
	}
	for (let i = 0; i < list.length; i++) {
		try {
			const mod = await importFunc(list[i]);
			plugins.push(mod);
		} catch (err) {
			console.error(`Failed to load plugin ${list[i]}`);
		}
	}
	return plugins;
}
