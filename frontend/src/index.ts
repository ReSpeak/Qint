import "./extensions";
import UiApp from "./UiApp.svelte";
import { mount } from 'svelte';
import { get } from "svelte/store";
import { app } from "./app";
import { ConnectData } from "./connect/uiConnect";
import debug from "debug";
import { backend } from "./backend/backend";
import { IS_TAURI } from "./util";
import "./index.css";

if (localStorage.getItem("debug") === null) debug.enable("error:*");

if (typeof DEBUG_UTIL !== "undefined") {
	(window as any).qint = app; // DEBUG
	(window as any).get = get; // DEBUG
	(window as any).debug = debug; // DEBUG
	(window as any).backend = backend; // DEBUG
	(window as any).debugset = (s: string) => {
		debug.enable(s);
		localStorage.setItem("debug", s);
	};
}

if (typeof BUILD_ENV !== "undefined")
	console.log("BUILD", BUILD_ENV, BUILD_DAT);
console.log(`Using ${backend.name} backend`);
app.settings.synth.init();

window.onbeforeunload = function (e: any) {
	if (!IS_TAURI) {
		// For debugging purposes (?)
		app.settings.synth.trySpeak("Goodbye");
		if (app.hasConnected && app.settings.app.askBeforeClosing) {
			if (e) {
				e.returnValue = true;
			}
			return true;
		}
	}

	app.close();
	return;
};

const loc = location.hash;
if (loc && loc !== "" && loc !== "#") {
	// Starts with #
	try {
		const conData = JSON.parse(decodeURIComponent(loc.substring(1)));
		for (const conData of conData) {
			app.connect(ConnectData.fromJSON(conData));
		}
	} catch (e) {
		console.error("Failed to connect to previous connection", e);
	}
}

const uiApp = mount(UiApp, {
	target: document.body,
});

export default uiApp;
