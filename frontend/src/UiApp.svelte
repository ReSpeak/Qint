<script lang="typescript">
	import { derived } from "svelte/store";
	import type { Writable } from "svelte/store";
	import Connected from "./Connected.svelte";
	import { ConnectionState } from "./connection";
	import { BUILD_ENV, BUILD_DAT } from "./util";
	import { app } from "./app";

	console.log("BUILD", BUILD_ENV, BUILD_DAT);

	let connections = app.connections;
	$: hasConnected = derived(
		$connections.map((c) => c.state) as [Writable<ConnectionState>],
		(states) =>
			states.some(
				(s) => s === ConnectionState.Connected || s === ConnectionState.ChannelListFinished
			)
	);

	(window as any).con = connections; // DEBUG

	window.onbeforeunload = function (e: any) {
		app.transientSettings.flush();

		// For debugging purposes
		if ($hasConnected) {
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
			// TODO Add new connection
			//connection.connect(getConnectFromString(decodeURIComponent(loc.substr(1))));
		} catch (e) {
			console.error("Failed to connect to previous connection", e);
		}
	}
</script>

<Connected {connections} hasConnected={$hasConnected} />

<style lang="scss" global>
	@import "@mdi/font/css/materialdesignicons";
	@import "katex/dist/katex.min";
	@import "highlight.js/styles/vs2015";

	@import "bulma/sass/base/_all";
	@import "bulma/sass/components/menu";
	@import "bulma/sass/components/tabs";
	@import "bulma/sass/elements/button";
	@import "bulma/sass/elements/icon";
	@import "bulma/sass/form/shared";
	@import "bulma/sass/form/input-textarea";
	@import "bulma/sass/form/select";
	@import "bulma/sass/form/tools";
	@import "bulma-slider/src/sass/index";
	// message box from bulma for error

	@import "./global";
</style>
