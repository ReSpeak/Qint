import "./extensions";
import App from "./App.svelte";
//import App from "./UiPlayground.svelte";
import { Connection } from "./connection";
import { get } from "svelte/store";

const connection = new Connection();
(window as any).con = connection; // DEBUG
(window as any).get = get; // DEBUG

const app = new App({
	props: {
		connection,
	},
	target: document.body,
});

export default app;
