<script lang="ts">
	import type { Writable } from "svelte/store";
	import { Connection } from "../connection";
	import { TsNotification } from "../notification";
	import UiNotification from "./UiNotification.svelte";

	export let notifications: Writable<[Connection, TsNotification][]>;
</script>

<ul class="notificationList">
	{#each $notifications as [connection, notification] (notification)}
		<li>
			<UiNotification {connection} {notification} />
		</li>
	{/each}
</ul>

<style lang="scss">
	.notificationList li {
		margin: 0.5em;
	}
</style>
