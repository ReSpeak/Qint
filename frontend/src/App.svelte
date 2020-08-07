<script lang="typescript">
	import { get } from "svelte/store";
	import Connect from "./connect/Connect.svelte";
	import Connected from "./Connected.svelte";
	import { ConnectionState, Connection } from "./connection";
	import { BUILD_ENV, BUILD_DAT } from "./util";
	console.log("BUILD", BUILD_ENV, BUILD_DAT);

	export let connection: Connection;
	$: state = connection.state;

	window.onbeforeunload = function(e: any) {
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
	
	@import "bulma/bulma";
	@import "bulma-slider/src/sass/index";
	@import "bulmaswatch/cyborg/overrides";

	@import "./global";
</style>
