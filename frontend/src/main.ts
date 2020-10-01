import "./extensions";
import UiApp from "./UiApp.svelte";
//import App from "./UiPlayground.svelte";
import { get } from "svelte/store";
import { app } from "./app";
import { getConnectFromString, BUILD_ENV, BUILD_DAT } from "./util";

(window as any).con = app.connections; // DEBUG
(window as any).get = get; // DEBUG
console.log("BUILD", BUILD_ENV, BUILD_DAT);

window.onbeforeunload = function (e: any) {
	app.transientSettings.flush();

	// For debugging purposes
	if (app.hasConnected) {
		if (e) {
			e.returnValue = true;
		}
		return true;
	}
	window.speechSynthesis.speak(new SpeechSynthesisUtterance("Goodbye"));
	return;
};

const loc = location.hash;
if (loc && loc !== "" && loc !== "#") {
	try {
		// Starts with #
		app.connect(getConnectFromString(decodeURIComponent(loc.substr(1))));
	} catch (e) {
		console.error("Failed to connect to previous connection", e);
	}
}

const uiApp = new UiApp({
	target: document.body,
});

export default uiApp;
