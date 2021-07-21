<script lang="ts">
	import LazyList from "../ui/container/LazyList.svelte";
	import { ListFetchDir } from "../ui/container/uiLazyList";
	import type { FetchResult } from "../ui/container/uiLazyList";
	import Icon from "../ui/icon/Icon.svelte";
	import UiMessageSearchResult from "./MessageSearchResult.svelte";
	import { EmptyMessageFetch, search } from "./uiSearch";
	import type { MessageSearchResult } from "./uiSearch";
	import debug from "debug";
	import { on } from "../util";
	const log = debug("SEARCH");

	export let filter: string;

	let searchList: LazyList<MessageSearchResult> | undefined;
	let searchError: unknown | undefined;

	$: {
		if (searchList) {
			if (filter.length >= 2) {
				if (searchList.sourceChanged)
					searchList.sourceChanged(ListFetchDir.New, ListFetchDir.Before);
			} else if (searchList.clear) {
				searchList.clear();
			}
		}
	}

	$: on(filter, resetError());

	function resetError() {
		searchError = undefined;
	}

	async function fetchMessageElements(
		idFrom: MessageSearchResult | undefined,
		dir: ListFetchDir
	): Promise<FetchResult<MessageSearchResult>> {
		searchError = undefined;
		try {
			let res;
			let canLoadAfterEnd = true;
			let canLoadBeforeStart = true;
			if (dir === ListFetchDir.Before && idFrom) {
				const start = Math.max(0, idFrom.id - 50);
				canLoadBeforeStart = start !== 0;
				res = await search(filter, true, start);
				res.messages = res.messages.slice(0, idFrom.id - start);
			} else {
				if (idFrom !== undefined) canLoadBeforeStart = idFrom.id !== 0;
				res = await search(filter, true, idFrom !== undefined ? idFrom.id + 1 : undefined);
				if (res.messages.length < 50) canLoadAfterEnd = false;
			}
			if (res.messages.length === 0) {
				canLoadBeforeStart = false;
				canLoadAfterEnd = false;
			}
			log("loading message search", idFrom, dir, "gives", res.messages);
			return {
				items: res.messages,
				canLoadAfterEnd,
				canLoadBeforeStart,
			};
		} catch (err) {
			console.error("Failed to load message search results", err);
			searchError = err;
			return EmptyMessageFetch;
		}
	}
</script>

<div class="searchResults">
	{#if searchError}
		<div>
			<article class="message is-danger">
				<div class="message-header">
					<p>Search failed</p>
					<button class="delete" aria-label="delete" on:click={resetError} />
				</div>
				<div class="message-body">{searchError}</div>
			</article>
		</div>
	{:else if filter.length >= 2}
		<LazyList
			bind:this={searchList}
			fetchElements={fetchMessageElements}
			suggestJumpStart={true}
			let:item>
			<div slot="loading" class="searchFiller">
				<span>Loading ...</span>
				<Icon name="orbit mdi-spin" />
			</div>
			<div slot="empty" class="searchFiller">No results ¯\_(ツ)_/¯</div>
			<UiMessageSearchResult content={item} />
		</LazyList>
	{:else}
		<div class="searchFiller" />
	{/if}
</div>

<style lang="scss">
	@import "../style/global_mixin";

	.searchResults {
		overflow: hidden;
		display: flex;
		flex-direction: column;
		justify-content: flex-end;
		line-height: initial;
		border-right: none;

		// The LazyList
		> :global(.lazyList) {
			flex: 1;
		}
	}

	.searchFiller {
		@extend %unselectable;
		width: 100%;
		height: 100%;
		padding: 0 1em 3em 1em;
		display: flex;
		flex-direction: column;
		justify-content: flex-end;
		align-items: center;
		color: gray;
		font-size: xx-large;
		white-space: nowrap;

		:global(.icon) {
			font-size: 72px;
		}
	}
</style>
