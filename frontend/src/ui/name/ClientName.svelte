<script lang="ts">
	import { ClientBase } from "../../bookBase";
	import { Client } from "../../book";
	import { getDataColor } from "../../util";
	import { app } from "../../app";
	import { Connection } from "../../connection";

	export let connection: Connection | undefined = undefined;
	export let client: ClientBase;

	let color: string = "";
	let name: string = "";

	function refreshClient(cl: ClientBase) {
		const data = cl.uid ?? cl.name;
		const name = cl.name;
		return [getDataColor(data), name];
	}

	function click() {
		if (client instanceof Client && connection !== undefined) {
			app.select(connection, client);
		}
	}

	$: [color, name] = refreshClient(client);
</script>

<span class="nameTag" tabindex="0" style="color: {color};" on:click={click}>{name}</span>

<style lang="scss">
	@import "./nametag";
</style>
