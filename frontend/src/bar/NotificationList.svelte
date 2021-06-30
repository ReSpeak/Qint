<script lang="ts">
	import type { Writable } from "svelte/store";
	import { Connection } from "../connection";
	import { TsNotification } from "../notifications";
	import Notification from "./Notification.svelte";

	export let notifications: Writable<[number, Connection, TsNotification][]>;
</script>

<ul class="notificationList">
	{#each $notifications as [i, connection, notification] (i)}
		<li>
			<Notification {connection} {notification} />
		</li>
	{/each}
</ul>

<style lang="scss">
	.notificationList li {
		margin: 0.5em;
	}
</style>
