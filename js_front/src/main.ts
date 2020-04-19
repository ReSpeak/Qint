import "@mdi/font/css/materialdesignicons.css";
import "bulma";
import "katex/dist/katex.min.css";
import "highlight.js/styles/vs2015.css";

import App from "./App.svelte";
import "./extensions";
import { Connection } from "./connection";

const connection = new Connection();
(window as any).con = connection; // DEBUG

const app = new App({
	props: {
		connection,
	},
	target: document.body,
});

export default app;
