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
		let data, name;
		data = cl.uid ?? cl.name;
		name = cl.name;
		return [getDataColor(data), name];
	}

	function click() {
		if (client instanceof Client && connection !== undefined) {
			app.select(connection, client);
		}
	}

	$: [color, name] = refreshClient(client);
</script>

<span tabindex="0" class="button noBut" style="color: {color};" on:click={click}>{name}</span>
