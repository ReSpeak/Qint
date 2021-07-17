<script lang="ts">
	import type { Writable } from "svelte/store";
	import Server from "../tree/ServerWrap.svelte";
	import StickyList from "../ui/container/StickyList.svelte";
	import StickySlot from "../ui/container/StickySlot.svelte";
	import SidebarSearchResults from "../search/SidebarSearchResults.svelte";
	import NotificationList from "./NotificationList.svelte";
	import { Connection } from "../connection";
	import { ConnectData } from "../connect/uiConnect";
	import { TsNotification } from "../notifications";

	export let connections: Writable<Connection[]>;
	export let notifications: Writable<[number, Connection, TsNotification][]>;
	export let filter: string;
	export let visible: boolean;
	export let showConnect: (data: ConnectData) => void;
</script>

<aside class="sidebar" class:hidden={!visible}>
	<StickyList>
		{#each $connections as connection (connection.backend.id)}
			<Server {connection} {filter} {showConnect} />
		{/each}

		{#if filter !== ""}
			<StickySlot>Search results</StickySlot>
			<SidebarSearchResults {filter} />
		{/if}

		<StickySlot>Notifications</StickySlot>
		<NotificationList {notifications} />
	</StickyList>
</aside>

<style lang="scss">
	.sidebar {
		display: inline-flex;
		flex-direction: column;
		background-color: $box-background-color;
		box-shadow: 3px 0 3px #0006;
		overflow-y: auto;
		z-index: 400; // Required to be over the chat and sticky headers

		:global(.searchResults) {
			max-height: calc(100vh - 7em);
		}
	}
</style>
