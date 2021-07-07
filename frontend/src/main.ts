import "./extensions";
import UiApp from "./UiApp.svelte";
//import App from "./Playground.svelte";
import { get } from "svelte/store";
import { app } from "./app";
import { ConnectData } from "./connect/uiConnect";
import debug from "debug";
import { backend } from "./backend/backend";

if (localStorage.getItem("debug") === null) debug.enable("error:*");

(window as any).qint = app; // DEBUG
(window as any).get = get; // DEBUG
(window as any).debug = debug; // DEBUG
(window as any).debugset = (s: string) => {
	debug.enable(s);
	localStorage.setItem("debug", s);
};
console.log("BUILD", BUILD_ENV, BUILD_DAT);
console.log(`Using ${backend.name} backend`);

window.onbeforeunload = function (e: any) {
	app.transientSettings.flush();

	// For debugging purposes (?)
	app.transientSettings.synth.trySpeak("Goodbye");
	if (app.hasConnected && app.transientSettings.app.askBeforeClosing) {
		if (e) {
			e.returnValue = true;
		}
		return true;
	}
	return;
};

const loc = location.hash;
if (loc && loc !== "" && loc !== "#") {
	// Starts with #
	try {
		const conDatas = JSON.parse(decodeURIComponent(loc.substring(1)));
		for (const conData of conDatas) {
			app.connect(ConnectData.fromJSON(conData));
		}
	} catch (e) {
		console.error("Failed to connect to previous connection", e);
	}
}

const uiApp = new UiApp({
	target: document.body,
});

export default uiApp;
