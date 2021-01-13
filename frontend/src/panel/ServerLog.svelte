<script lang="typescript">
	import { onDestroy } from "svelte";
	import UiLazyList from "../ui/UiLazyList.svelte";
	import UiChangeResult from "../ui/UiChangeResult.svelte";
	import Icon from "../ui/Icon.svelte";
	import { ListFetchDir } from "../ui/lazyList";
	import { Connection } from "../connection";
	import { on } from "../util";
	import type { ResultDetails } from "../backend/ws";
	import { LogEntry, ServerLogState } from "./serverLog";

	export let connection: Connection;

	let logList: UiLazyList;
	let fetchError: ResultDetails | undefined;
	let state = new ServerLogState(connection);

	$: on(connection, connectionChanged());

	function connectionChanged() {
		fetchError = undefined;
		state.unsubscribe();
		state = new ServerLogState(connection);
		if (logList)
			logList.sourceChanged(ListFetchDir.New, ListFetchDir.After);
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
				canLoadAfterEnd: false
			};
		}
	}

	onDestroy(() => {
		state.unsubscribe();
	});
</script>

<div class="serverLog">
	{#if fetchError}
		<div class="notification is-danger">
			<button
				class="toolbutton is-small"
				style="float: right;"
				on:click={() => fetchError = undefined}>
				<Icon name="close" />
			</button>
			<UiChangeResult result={fetchError} />
		</div>
	{/if}
	<UiLazyList
		bind:this={logList}
		{fetchElements}
		suggestJumpEnd={true}
		let:item>
		<div slot="loading" class="logFiller">
			<span>Loading ...</span>
			<Icon name="orbit mdi-spin" />
		</div>
		<div slot="empty" class="logFiller"></div>
		<div class="logLine">{item.log}</div>
	</UiLazyList>
</div>

<style lang="scss">
	.serverLog {
		overflow: hidden;
		display: flex;
		flex-direction: column;
		justify-content: flex-end;
		line-height: initial;
		border-right: none;

		// The LazyList
		> :global(.lazyList) {
			flex: 1;

			> :global(.lazyListView) {
				overflow-x: scroll;
			}
		}
	}
</style>
