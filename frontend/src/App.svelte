<script lang="typescript">
	import { get } from "svelte/store";
	import Connect from "./connect/Connect.svelte";
	import Connected from "./Connected.svelte";
	import { ConnectionState, Connection } from "./connection";
	import { BUILD_ENV, BUILD_DAT } from "./util";
	import { transientSettings } from "./transientSettings";

	console.log("BUILD", BUILD_ENV, BUILD_DAT);

	export let connection: Connection;
	$: state = connection.state;

	window.onbeforeunload = function (e: any) {
		transientSettings.flush();

		let s = get(state);
		// For debugging puproses ?
		window.speechSynthesis.speak(new SpeechSynthesisUtterance("Goodbye"));
		if (s === ConnectionState.Connected || s === ConnectionState.ChannelListFinished) {
			if (e) {
				e.returnValue = true;
			}
			return true;
		}
		return;
	};
</script>

{#if $state !== ConnectionState.Connected && $state !== ConnectionState.ChannelListFinished}
	<Connect {connection} />
{:else}
	<Connected {connection} />
{/if}

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
