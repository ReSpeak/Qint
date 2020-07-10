<script lang="typescript">
	import { sleep, assert, binarySearchBy, BinarySearchResult } from "../util";
	import { tick, onMount } from "svelte";
	import { writable } from "svelte/store";
	import * as svst from "svelte/store";
	import { ListFetchDir, FetchResult } from "./lazyList";

	// Dummy class to have nice typing for our 'generic' parameter T which
	// represents the element type.
	class T {}

	// *** State+Export variables ***

	export let enableFetching: boolean = true;
	let canLoadAfterEnd: boolean = true;
	let canLoadBeforeStart: boolean = true;
	let arrowHidden = true;
	let loadAnchored: ListFetchDir | undefined = undefined;

	// the data elements held by this list
	let elems: T[] = [];

	let pxBeforeLoad = 500; // TODO? could be adjusted dynamically
	/** The minimum amout of items that must be at least `minPxDistanceToRemove`
	 * will be removed when out of view */
	let minItemsToRemove = 20;
	/** How far the item at index `minItemsToRemove` has to be out of view to be removed */
	let minPxDistanceToRemove = 1500;

	declare let holdIdStart: T | undefined;
	declare let holdIdEnd: T | undefined;
	// the lowest and highest _included_ id currently in the list
	$: holdIdStart = elems.length !== 0 ? elems[0] : undefined;
	$: holdIdEnd = elems.length !== 0 ? elems[elems.length - 1] : undefined;

	// The holding list element which has the scrollbar
	let pan: HTMLElement;
	// Utility holder to calculate `scrollDiff`
	let lastScrollPos: number = 0;
	// In which direction and how far the content has scrolled since last check
	// >0 down, <0 up
	let scrollDiff = 0;
	// prevent async weirdness by only allowing one async task
	let fetchTask: Promise<void> | undefined;

	// *** Export functions ***

	export function clear() {
		elems = [];
		canLoadAfterEnd = false;
		canLoadBeforeStart = false;
	}

	export function sourceChanged(dir: ListFetchDir, anchor?: ListFetchDir) {
		loadAnchored = anchor;
		switch (dir) {
			case ListFetchDir.New:
				canLoadBeforeStart = true;
				canLoadAfterEnd = true;
				break;
			case ListFetchDir.Before:
				canLoadBeforeStart = true;
				break;
			case ListFetchDir.After:
				canLoadAfterEnd = true;
				break;
		}
		start_fill();
	}

	export function jumpTo(dir: ListFetchDir, target?: T) {
		switch (dir) {
			case ListFetchDir.Before:
				if (!canLoadBeforeStart) {
					scrollToStart();
					break;
				}
				clear();
				sourceChanged(ListFetchDir.New, ListFetchDir.Before);
				break;

			case ListFetchDir.After:
				if (!canLoadAfterEnd) {
					scrollToEnd();
					break;
				}
				clear();
				sourceChanged(ListFetchDir.New, ListFetchDir.After);
				break;

			case ListFetchDir.New:
				assert(target, "target must be given when jumping to a new item");
				// TODO
				break;
		}
	}

	export let fetchElements!: (id: T | undefined, dir: ListFetchDir) => Promise<FetchResult<T>>;

	// Require the minimum distance before deleting an item to be higher
	// than the minimum size the list wants to buffer.
	// Otherwise we might end in an loop of adding and removing a side.
	assert(
		minPxDistanceToRemove > pxBeforeLoad,
		"Distance to delete must be greater than distance to load"
	);

	// *** Private functions ***

	function scrollToStart() {
		pan.scrollTop = 0;
	}
	function scrollToEnd() {
		pan.scrollTop = pan.scrollHeight - pan.clientHeight;
	}

	function handle_scroll(e: MouseEvent) {
		// console.log(
		// 	pan.scrollHeight, // complete content
		// 	pan.scrollTop,    // current scroll position
		// 	pan.scrollTopMax, // max scoll position
		// 	pan.offsetHeight, // container height
		// 	pan.clientHeight, // inner view height (after subtracting border/padding)
		// );
		// ! ELEM.offsetTop   // is the bottom of a element measured from the top of the container
		// ! clientHeight + scrollTopMax === scrollHeight
		scrollDiff = pan.scrollTop - lastScrollPos;
		lastScrollPos = pan.scrollTop;

		arrowHidden = pan.scrollTop >= pan.scrollHeight - pan.clientHeight;

		start_fill();
	}

	function start_fill() {
		if (fetchTask) {
			return;
		}
		fetchTask = fill_loop();
	}

	async function fill_loop() {
		const loadMaxBeforeError = 50;
		for (let i = 0; i <= loadMaxBeforeError; i++) {
			if (i === loadMaxBeforeError) throw Error("yah, thats a loop");
			if (await fill_body()) break;
		}
		fetchTask = undefined;
	}

	/**
	 * Will repeatedly fetch list chunks from the source until no more elements
	 * can be fetched or the list is full enough.
	 * @returns true when the list is satisfied with loading data
	 */
	async function fill_body(): Promise<boolean> {
		if ((!canLoadAfterEnd && !canLoadBeforeStart) || !enableFetching) return true;

		if (holdIdStart === undefined || holdIdEnd === undefined) {
			await load(ListFetchDir.New);
			return false;
		}

		const distFromTop = pan.scrollTop;
		const pan_scrollTopMax = pan.scrollHeight - pan.clientHeight;
		const distFromBot = pan_scrollTopMax - pan.scrollTop;

		const wantFetchStart = distFromTop < pxBeforeLoad && scrollDiff <= 0;
		const wantFetchEnd = distFromBot < pxBeforeLoad && scrollDiff >= 0;

		if (wantFetchStart && canLoadBeforeStart) {
			// console.log("want start", holdIdStart);
			await load(ListFetchDir.Before, holdIdStart);
			return false;
		} else if (wantFetchEnd && canLoadAfterEnd) {
			// console.log("want end", holdIdEnd);
			await load(ListFetchDir.After, holdIdEnd);
			return false;
		} else {
			return true;
		}
	}

	/**
	 * Checks the direction of the request. Fetches the data from the source
	 * and applies them into the list.
	 * This will fetch one block only.
	 */
	async function load(dir: ListFetchDir.After | ListFetchDir.Before, from: T): Promise<void>;
	async function load(dir: ListFetchDir.New, from?: T): Promise<void>;
	async function load(dir: ListFetchDir, from?: T): Promise<void> {
		assert(
			dir === ListFetchDir.New || from !== undefined,
			"Invalid load request. from:",
			from,
			"dir:",
			ListFetchDir[dir]
		);
		const result = await fetchElements(from, dir);
		assert(result, "result from fetch is not valid");
		//console.log("fetchElements result", result);

		if (dir === ListFetchDir.Before) {
			if (result.items.length === 0)
				assert(
					!result.canLoadBeforeStart,
					"Empty fetch result, but can still load",
					dir,
					result
				);
			canLoadBeforeStart = result.canLoadBeforeStart;
		} else if (dir === ListFetchDir.After) {
			if (result.items.length === 0)
				assert(
					!result.canLoadAfterEnd,
					"Empty fetch result, but can still load",
					dir,
					result
				);
			canLoadAfterEnd = result.canLoadAfterEnd;
		} else {
			if (result.items.length === 0)
				assert(
					!result.canLoadBeforeStart && !result.canLoadAfterEnd,
					"Empty fetch result, but can still load",
					dir,
					result
				);
			canLoadBeforeStart = result.canLoadBeforeStart;
			canLoadAfterEnd = result.canLoadAfterEnd;
		}
		await applyElements(result.items, dir);
	}

	/**
	 * Utility method to replace the current list with a new list without
	 * changing the scroll position.
	 */
	async function modifyElems(newElems: T[]) {
		const lastScrollHeight = pan.scrollHeight;
		const lastScrollTop = pan.scrollTop;
		//console.log("Before change scrollHeight", lastScrollHeight, ", scrollTop", pan.scrollTop);
		elems = newElems;
		await tick();
		//console.log("In change scrollTop", pan.scrollTop);
		const scrollAdjust = pan.scrollHeight - lastScrollHeight;
		pan.scrollTop = lastScrollTop + scrollAdjust;
		lastScrollPos += scrollAdjust;
		// console.log("modifyElems scrollHeight", pan.scrollHeight, "scrollTop", pan.scrollTop, "scrollAdjust", scrollAdjust);
	}

	/**
	 * Checks if a block at the end of the list is far enough out of view and
	 * removes it.
	 */
	async function tryTrimEnd() {
		if (elems.length <= minItemsToRemove) return;
		await tick();
		
		const childList = pan.querySelectorAll<HTMLElement>(".scrollPane > .lazyListElement");
		assert(childList.length === elems.length, "HTML node count does not match elements count");
		
		const distFn = (e: HTMLElement) => {
			// The top of the element within our list (unscrolled)
			const topStaticOffset = e.offsetTop - e.offsetHeight;
			// The top of the element without our list (with scroll offset)
			const topCurrentOffset = topStaticOffset - pan.scrollTop;
			// The distance from the top of the element to the bottom of our list
			const distFromBottom = topCurrentOffset - pan.offsetHeight;
			// (Boolean) If the distance is further than our remove distance threshold.
			// < 0 false | > 0 true
			const diffToRemove = distFromBottom - minPxDistanceToRemove;
			return diffToRemove;
		};

		const trimSearchResult = binarySearchBy(childList, distFn, 0, childList.length - minItemsToRemove + 1);
		const child = childList[trimSearchResult.index];
		const dist = distFn(child);
		console.log("tryTrimEnd", trimSearchResult, "would trim", child, "with dist", dist);

		if (dist > 0) {
			// modification is at the end => safe
			elems = elems.slice(0, trimSearchResult.index);
			canLoadAfterEnd = true;
		}
	}

	/**
	 * Checks if a block at the start of the list is far enough out of view and
	 * removes it.
	 */
	async function tryTrimStart() {
		if (elems.length <= minItemsToRemove) return;
		await tick();
		let childList = pan.querySelectorAll<HTMLElement>(".scrollPane > .lazyListElement");
		assert(childList.length === elems.length, "HTML node count does not match elements count");

		const distFn = (e: HTMLElement) => {
			// The bottom of the element within our list (unscrolled)
			const bottomStaticOffset = e.offsetTop;
			// The top of the element without our list (with scroll offset)
			const bottomCurrentOffset = bottomStaticOffset - pan.scrollTop;
			// The distance from bottom of the element to the top of our list
			const distFromTop = 0 - bottomCurrentOffset;
			// (Boolean) If the distance is further than our remove distance threshold.
			// < 0 false | > 0 true
			const diffToRemove = minPxDistanceToRemove - distFromTop;
			return diffToRemove;
		};

		const trimSearchResult = binarySearchBy(childList, distFn, minItemsToRemove - 1, undefined);
		const child = childList[trimSearchResult.index];
		const dist = distFn(child);
		console.log("tryTrimStart", trimSearchResult, "would trim", child, "with dist", dist);

		if (dist > 0) {
			// mofification at start => helper
			await modifyElems(elems.slice(trimSearchResult.index));
			canLoadBeforeStart = true;
		}
	}

	/**
	 * Appends/Prepends or replaces the list with the new passed list.
	 */
	async function applyElements(newElems: T[], dir: ListFetchDir) {
		// TODO:not sure, but I think add + trim could be done in one step
		switch (dir) {
			case ListFetchDir.After:
				// This case adds elements at the end => trim start
				elems = [...elems, ...newElems]; // modification is at the end => safe
				await tryTrimStart();
				break;

			case ListFetchDir.Before:
				// This case adds elements at the start => trim end
				await modifyElems([...newElems, ...elems]); // mofification at start => helper
				await tryTrimEnd();
				break;

			case ListFetchDir.New:
				elems = newElems;
				if (loadAnchored === ListFetchDir.Before) {
					scrollToStart();
				} else if (loadAnchored === ListFetchDir.After) {
					await tick();
					scrollToEnd();
				}
				break;

			default:
				throw new Error("Unhandled direction case");
		}
	}

	onMount(() => {
		start_fill();
		pan.onresize = start_fill;
	});
</script>

<svelte:options accessors />
<div class="lazyList">
	<div class="lazyListView" bind:this="{pan}" on:scroll="{handle_scroll}">
		<div class="scrollPane">
			{#each elems as item (item)}
				<div class="lazyListElement">
					<slot {item} />
				</div>
			{/each}
		</div>
	</div>
	<button class="arrow-down" class:arrowHidden on:click="{() => jumpTo(ListFetchDir.After)}">
		<div></div>
	</button>
</div>

<style lang="scss">
	.lazyList {
		position: relative;
		overflow: hidden;
	}

	.lazyListView {
		overflow-x: hidden;
		overflow-y: scroll;
		height: 100%;
	}

	// Jump start end buttons

	.arrow-down,
	.arrow-up {
		position: absolute;
		right: 2em;
		bottom: 1.5em;
		display: inline-block;
		background: #ccc;
		border-radius: 100%;
		padding: 0.8em;
		border: none;
		cursor: pointer;
		z-index: 3;

		transition-duration: 0.2s;
		transition-property: all;
	}

	.arrow-down:hover,
	.arrow-up:hover {
		background: #eee;
	}

	.arrowHidden {
		bottom: -5em;
	}

	.arrow-down > div,
	.arrow-up > div {
		border-left: 2px solid #222;
		border-top: 2px solid #222;
		width: 1em;
		height: 1em;
	}

	.arrow-down > div {
		transform: rotate(-135deg) translate(20%, 20%);
	}

	.arrow-up > div {
		transform: rotate(45deg) translate(20%, 20%);
	}
</style>
