import "./extensions";
import UiApp from "./UiApp.svelte";
//import App from "./UiPlayground.svelte";
import { get } from "svelte/store";

(window as any).get = get; // DEBUG

const app = new UiApp({
	target: document.body,
});

export default app;
