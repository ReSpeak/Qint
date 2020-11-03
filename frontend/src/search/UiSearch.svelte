<script lang="typescript">
	import UiLazyList from "../ui/UiLazyList.svelte";
	import { ListFetchDir } from "../ui/lazyList";
	import type { FetchResult } from "../ui/lazyList";
	import Icon from "../ui/Icon.svelte";
	import UiSearchResult from "./UiSearchResult.svelte";
	import { EmptyFetch, search } from "./search";
	import type { SearchResult } from "./search";

	export let filter: string;

	let searchList: UiLazyList | undefined;
	let searchError: unknown | undefined;
	let resultCount = 0;

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

	async function fetchElements(idFrom: SearchResult | undefined, dir: ListFetchDir): Promise<FetchResult<SearchResult>> {
		searchError = undefined;
		try {
			let res;
			let canLoadAfterEnd = true;
			let canLoadBeforeStart = true;
			if (dir === ListFetchDir.Before && idFrom) {
				const start = Math.max(0, idFrom.id - 50);
				canLoadBeforeStart = start !== 0;
				res = await search(filter, start);
				res.results = res.results.slice(0, idFrom.id - start);
				resultCount = res.count;
			} else {
				if (idFrom !== undefined)
					canLoadBeforeStart = idFrom.id !== 0;
				res = await search(filter, idFrom !== undefined ? idFrom.id + 1 : undefined);
				resultCount = res.count;
				if (res.results.length < 50)
					canLoadAfterEnd = false;
			}
			console.log("loading", idFrom, dir, "gives", res.results);
			return {
				items: res.results,
				canLoadAfterEnd,
				canLoadBeforeStart,
			};
		} catch (err) {
			console.error("Failed to load search results", err);
			searchError = err;
			return EmptyFetch;
		}
	}
</script>

<div class="searchResults">
	{#if searchError}
		<div>
			<article class="message is-danger">
				<div class="message-header">
					<p>Error</p>
				</div>
				<div class="message-body">Failed to search</div>
			</article>
		</div>
	{:else if filter.length >= 2}
		<UiLazyList
			bind:this={searchList}
			{fetchElements}
			suggestJumpStart={true}
			let:item>
			<div slot="loading" class="searchFiller">
				<span>Loading ...</span>
				<Icon name="orbit mdi-spin" />
			</div>
			<div slot="empty" class="searchFiller">No results ¯\_(ツ)_/¯</div>
			<UiSearchResult content={item} />
		</UiLazyList>
	{:else}
		<div class="searchFiller"></div>
	{/if}
</div>

<style lang="scss">
	.searchResults {
		max-height: 100%;
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
