<script lang="ts">
	import { onMount } from "svelte";
	import LazyList from "../ui/container/LazyList.svelte";
	import ChangeResult from "../ui/specialized/ChangeResult.svelte";
	import Icon from "../ui/icon/Icon.svelte";
	import { ListFetchDir } from "../ui/container/uiLazyList";
	import { Connection } from "../connection";
	import { on } from "../util";
	import type { ResultDetails } from "../backend/ws";
	import { LogEntry, ServerLogState } from "./uiServerLog";

	export let connection: Connection;

	let logList: LazyList<LogEntry>;
	let fetchError: ResultDetails | undefined;
	let state = new ServerLogState(connection);

	$: on(connection, connectionChanged());

	function connectionChanged() {
		fetchError = undefined;
		state.unsubscribe();
		state = new ServerLogState(connection);
		if (logList) logList.sourceChanged(ListFetchDir.New, ListFetchDir.After);
	}

	async function fetchElements(idFrom: LogEntry | undefined, dir: ListFetchDir) {
		fetchError = undefined;
		try {
			return await state.fetchElements(idFrom, dir);
		} catch (err) {
			fetchError = err;
			return {
				items: [],
				canLoadBeforeStart: false,
				canLoadAfterEnd: false,
			};
		}
	}

	onMount(() => {
		connectionChanged();
		return () => state.unsubscribe();
	});
</script>

<div class="serverLog">
	{#if fetchError}
		<div class="notification is-danger">
			<button
				class="toolbutton is-small"
				style="float: right;"
				on:click={() => (fetchError = undefined)}
			>
				<Icon name="close" />
			</button>
			<ChangeResult result={fetchError} />
		</div>
	{/if}
	<LazyList bind:this={logList} {fetchElements} suggestJumpEnd={true} let:item>
		<div slot="loading" class="logFiller">
			<span>Loading ...</span>
			<Icon name="orbit mdi-spin" />
		</div>
		<div slot="empty" class="logFiller" />
		<div class="logLine">{item.log}</div>
	</LazyList>
</div>

<style lang="scss">
	@use "../index.scss" as *;
	@import "../style/global_mixin";

	.serverLog {
		@include lazylistContainer;
	}
</style>
