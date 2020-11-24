import "./extensions";
import UiApp from "./UiApp.svelte";
//import App from "./UiPlayground.svelte";
import { get } from "svelte/store";
import { app } from "./app";
import { BUILD_ENV, BUILD_DAT } from "./util";
import { ConnectData } from "./connect/connect";
import debug from "debug";

if (localStorage.getItem("debug") === null)
	debug.enable("error:*");

(window as any).qint = app; // DEBUG
(window as any).get = get; // DEBUG
(window as any).debug = debug; // DEBUG
console.log("BUILD", BUILD_ENV, BUILD_DAT);

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
		const conDatas = JSON.parse(decodeURIComponent(loc.substr(1)));
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

// Hot Module Replacement (HMR) - Remove this snippet to remove HMR.
// Learn more: https://www.snowpack.dev/#hot-module-replacement
if ((import.meta as any).hot) {
	console.log("Aww, that's hot", import.meta);
	(import.meta as any).hot.accept();
	(import.meta as any).hot.dispose(() => {
		uiApp.$destroy();
		// Disconnect previous connections
		app.close();
	});
}
