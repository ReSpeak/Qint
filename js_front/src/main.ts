import App from "./App.svelte";
//import App from "./UiPlayground.svelte";
import "./extensions";
import { Connection } from "./connection";
import { get } from "svelte/store";
import hljs from "highlight.js";

const connection = new Connection();
(window as any).con = connection; // DEBUG
(window as any).get = get; // DEBUG
(window as any).hljs = hljs; // DEBUG

const app = new App({
	props: {
		connection,
	},
	target: document.body,
});

export default app;
