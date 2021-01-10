<script lang="typescript">
	import { CustomIntersectionObserver, ListFetchDir } from "./lazyList";
	import type { FetchResult } from "./lazyList";
	import { assert, binarySearchByKey, debounced } from "../util";
	import { createEventDispatcher, tick, onMount } from "svelte";
	import ResizeObserver from "resize-observer-polyfill";
	import debug from "debug";
	const log = debug("LL");
	const dispatch = createEventDispatcher<{ viewchanged: { first?: T; last?: T } }>();

	// Dummy class to have nice typing for our 'generic' parameter T which
	// represents the element type.
	type T = any;

	// The golden handbook for js/css:
	// - pan.scrollHeight, // complete content
	// - pan.scrollTop,    // current scroll position
	// - pan.scrollTopMax, // max scoll position
	// - pan.offsetHeight, // container height
	// - pan.clientHeight, // inner view height (after subtracting border/padding)
	// ! ELEM.offsetTop    // is the top of a element measured from the top of the first position:relative parent.
	// ! clientHeight + scrollTopMax === scrollHeight

	// *** State+Export variables ***

	export let enableFetching: boolean = true;
	export let suggestJumpStart: boolean = false;
	export let suggestJumpEnd: boolean = false;
	export let notifyViewChanged: boolean = false;
	let canLoadBeforeStart: boolean = true;
	let canLoadAfterEnd: boolean = true;
	let showJumpStart: boolean;
	let showJumpEnd: boolean;
	let loadAnchored: ListFetchDir | undefined = undefined;
	let lastViewStart: number | undefined;
	let lastViewEnd: number | undefined;
	let visObs: CustomIntersectionObserver;

	// the data elements held by this list
	let elems: T[] = [];

	let pxBeforeLoad = 500; // TODO? could be adjusted dynamically
	/** The minimum amout of items that must be at least `minPxDistanceToRemove`
	 * will be removed when out of view */
	let minItemsToRemove = 20;
	/** How far the item at index `minItemsToRemove` has to be out of view to be removed */
	let minPxDistanceToRemove = 1500;
	// The holding list element which has the scrollbar
	let pan: HTMLElement;
	let scrollPane: HTMLElement;
	let mounted = false;
	// prevent async weirdness by only allowing one async task
	let fetchTask: Promise<void> | undefined;

	// *** Export functions ***

	export function clear() {
		elems = [];
		visObs.clear();
		canLoadAfterEnd = false;
		canLoadBeforeStart = false;
		showJumpStart = false;
		showJumpEnd = false;
		lastViewStart = undefined;
		lastViewEnd = undefined;
	}

	export function sourceChanged(dir: ListFetchDir, anchor?: ListFetchDir) {
		loadAnchored = anchor;
		switch (dir) {
			case ListFetchDir.New:
				clear();
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
				sourceChanged(ListFetchDir.New, ListFetchDir.Before);
				break;

			case ListFetchDir.After:
				if (!canLoadAfterEnd) {
					scrollToEnd();
					break;
				}
				sourceChanged(ListFetchDir.New, ListFetchDir.After);
				break;

			case ListFetchDir.New:
				assert(target, "target must be given when jumping to a new item");
				// TODO
				break;
		}
	}

	type fetchFun<TL = any> = (id: TL | undefined, dir: ListFetchDir) => Promise<FetchResult<TL>>;
	export let fetchElements: fetchFun;

	// Require the minimum distance before deleting an item to be higher
	// than the minimum size the list wants to buffer.
	// Otherwise we might end in an loop of adding and removing a side.
	assert(
		minPxDistanceToRemove > pxBeforeLoad,
		"Distance to delete must be greater than distance to load"
	);
	assert(minItemsToRemove >= 1, "Minimum items to remove must be >=1");

	// *** Private functions ***

	function scrollToStart() {
		if (!mounted) return;
		pan.scrollTop = 0;
	}
	function scrollToEnd() {
		if (!mounted) return;
		pan.scrollTop = pan.scrollHeight - pan.clientHeight;
	}
	function getFirstElem() {
		return elems.length !== 0 ? elems[0] : undefined;
	}
	function getLastElem() {
		return elems.length !== 0 ? elems[elems.length - 1] : undefined;
	}

	function getHtmlElements(): ArrayLike<HTMLElement> {
		const childList = (scrollPane.children as any) as ArrayLike<HTMLElement>;
		assert(childList.length === elems.length, "HTML node count does not match elements count");
		return childList;
	}

	function start_fill() {
		if (fetchTask || !mounted) {
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

		const holdIdStart = getFirstElem();
		const holdIdEnd = getLastElem();
		if (holdIdStart === undefined || holdIdEnd === undefined) {
			await load(ListFetchDir.New);
			return false;
		}

		const distFromTop = pan.scrollTop;
		const pan_scrollTopMax = pan.scrollHeight - pan.clientHeight;
		const distFromBot = pan_scrollTopMax - pan.scrollTop;

		const wantFetchStart = distFromTop <= pxBeforeLoad;
		const wantFetchEnd = distFromBot <= pxBeforeLoad;

		if (wantFetchStart && canLoadBeforeStart) {
			log("want start %o", holdIdStart);
			await load(ListFetchDir.Before, holdIdStart);
			return false;
		} else if (wantFetchEnd && canLoadAfterEnd) {
			log("want end %o", holdIdEnd);
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
		log("fetchElements result %o", result);

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
	async function modifyElems(newElems: T[], isAtTop: boolean) {
		if (!mounted) return;
		const lastScrollHeight = pan.scrollHeight;
		const lastScrollTop = pan.scrollTop;
		log("Before change scrollHeight:%d scrollTop:%d", lastScrollHeight, pan.scrollTop);

		triggerResizing();

		elems = newElems;
		await tick();

		const childList = getHtmlElements();
		visObs.observeReplace(childList);
		lastViewStart = undefined;
		lastViewEnd = undefined;

		if (isAtTop) {
			const scrollAdjust = pan.scrollHeight - lastScrollHeight;
			pan.scrollTop = lastScrollTop + scrollAdjust;
			log("isAtTop sH:%d sT:%d adjust:%d", pan.scrollHeight, pan.scrollTop, scrollAdjust);
		}
	}

	/**
	 * Checks if a block at the end of the list is far enough out of view and
	 * removes it.
	 */
	async function tryTrimEnd() {
		if (elems.length <= minItemsToRemove) return;
		if (elems.length === 0) return;
		const childList = getHtmlElements();

		const distFn = (e: HTMLElement) => {
			// The top of the element within our list (unscrolled)
			const topStaticOffset = e.offsetTop;
			// The top of the element without our list (with scroll offset)
			const topCurrentOffset = topStaticOffset - pan.scrollTop;
			return topCurrentOffset;
		};

		const removeDistance = pan.offsetHeight + minPxDistanceToRemove;
		const removeIndex = childList.length - minItemsToRemove;
		const res = binarySearchByKey(childList, removeDistance, distFn, 0, removeIndex + 1);

		log("tryTrimEnd %o %o", res, res.index <= removeIndex);
		if (res.index <= removeIndex) {
			// modification is at the end => safe
			await modifyElems(elems.slice(0, res.index), false);
			canLoadAfterEnd = true;
		}
	}

	/**
	 * Checks if a block at the start of the list is far enough out of view and
	 * removes it.
	 */
	async function tryTrimStart() {
		if (elems.length <= minItemsToRemove) return;
		if (elems.length === 0) return;
		const childList = getHtmlElements();

		const distFn = (e: HTMLElement) => {
			// The bottom of the element within our list (unscrolled)
			const bottomStaticOffset = e.offsetTop + e.offsetHeight;
			// The top of the element without our list (with scroll offset)
			const bottomCurrentOffset = bottomStaticOffset - pan.scrollTop;
			return bottomCurrentOffset;
		};

		const res = binarySearchByKey(
			childList,
			-minPxDistanceToRemove,
			distFn,
			minItemsToRemove - 1,
			undefined
		);

		log("tryTrimStart %o %o", res, res.index >= minItemsToRemove);
		// We are trimming index-1 since the result item is the first element
		// that is _smaller_ than our threshold distance.
		if (res.index >= minItemsToRemove) {
			// mofification at start => helper
			await modifyElems(elems.slice(res.index - 1), true);
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
				if (newElems.length > 0) {
					await modifyElems([...elems, ...newElems], false); // modification is at the end => safe
					await tryTrimStart();
				}
				break;

			case ListFetchDir.Before:
				// This case adds elements at the start => trim end
				if (newElems.length > 0) {
					await modifyElems([...newElems, ...elems], true); // mofification at start => helper
					await tryTrimEnd();
				}
				break;

			case ListFetchDir.New:
				elems = newElems;
				if (loadAnchored === ListFetchDir.Before) {
					scrollToStart();
				} else if (loadAnchored === ListFetchDir.After) {
					//await tick();
					//scrollToEnd();
					docked = true;
				}
				break;

			default:
				throw new Error("Unhandled direction case");
		}
		recheckJumpButton();
	}

	// Resize/Scroll events
	let docked = false;
	let resizing = false;

	let lockElem: HTMLElement | undefined;
	let lockPos: number | undefined;

	const resizingTimeout = debounced(
		() => {
			resizing = false;
			start_fill();
		},
		200,
		{ resetOnCall: true }
	);

	function triggerResizing() {
		if (docked) {
			pan.scrollTop = pan.scrollHeight;
		} else {
			if (lockElem && lockPos !== undefined) {
				pan.scrollTop = lockElem.offsetTop - lockPos;
				log("jumping %d %d %d", lockElem.offsetTop - lockPos, lockElem.offsetTop, lockPos);
			}
		}

		resizing = true;
		resizingTimeout();
	}

	function handle_scroll() {
		if (!resizing) {
			setLockPos();
			if (pan.scrollTop === pan.scrollHeight - pan.clientHeight) {
				//if (docked === false) log("docked");
				docked = true;
			} else {
				//if (docked === true) log("undocked");
				docked = false;
			}
			start_fill();
		}
		recheckJumpButton();
	}

	function recheckJumpButton() {
		if (elems.length === 0) {
			showJumpStart = false;
			showJumpEnd = false;
		} else {
			// Update jump up/down button visibility
			const isScrollable = pan.scrollHeight > pan.clientHeight;
			if (suggestJumpStart) {
				const isScrolledToStart = pan.scrollTop <= 0;
				showJumpStart = isScrollable && (!isScrolledToStart || canLoadBeforeStart);
			}
			if (suggestJumpEnd) {
				const isScrolledToEnd = pan.scrollTop >= pan.scrollHeight - pan.clientHeight;
				showJumpEnd = isScrollable && (!isScrolledToEnd || canLoadAfterEnd);
			}
		}
	}

	function setLockPos(elem?: HTMLElement) {
		if (resizing) return;
		if (elem) {
			lockElem = elem;
		} else if (!lockElem) {
			return;
		}
		lockPos = lockElem.offsetTop - pan.scrollTop;
	}

	function onIntersectionChanged(elem: IntersectionObserverEntry[]): void {
		let oldIdStart = lastViewStart;
		let oldIdEnd = lastViewEnd;
		let newLockElem: HTMLElement | undefined;
		for (const e of elem) {
			let htmlElem = e.target as HTMLElement;
			let elemId = Number(htmlElem.dataset.index);
			if (e.isIntersecting) {
				if (lastViewStart === undefined || elemId < lastViewStart) {
					newLockElem = htmlElem;
					lastViewStart = elemId;
				} else if (lastViewEnd === undefined || elemId > lastViewEnd) {
					lastViewEnd = elemId;
				}
			} else {
				if (
					e.boundingClientRect.bottom < e.rootBounds!.top &&
					(lastViewStart === undefined || elemId + 1 > lastViewStart)
				) {
					newLockElem = htmlElem.nextElementSibling
						? (htmlElem.nextElementSibling as HTMLElement)
						: htmlElem;
					lastViewStart = elemId + 1;
				} else if (
					e.boundingClientRect.top > e.rootBounds!.bottom &&
					(lastViewEnd === undefined || elemId - 1 < lastViewEnd)
				) {
					lastViewEnd = elemId - 1;
				}
			}
		}

		const startChanged = oldIdStart !== lastViewStart;
		const endChanged = oldIdEnd !== lastViewEnd;
		if (startChanged && newLockElem !== undefined) {
			setLockPos(newLockElem);
		}
		if (notifyViewChanged && (startChanged || endChanged)) {
			dispatch("viewchanged", {
				first: lastViewStart !== undefined ? elems[lastViewStart] : undefined,
				last: lastViewEnd !== undefined ? elems[lastViewEnd] : undefined,
			});
		}
	}

	onMount(() => {
		start_fill();

		const resizeObserver = new ResizeObserver(() => triggerResizing());
		resizeObserver.observe(pan);
		resizeObserver.observe(scrollPane);

		visObs = new CustomIntersectionObserver(onIntersectionChanged, {
			root: pan,
			threshold: 0,
		});

		mounted = true;
		return () => {
			resizeObserver.disconnect();
			visObs.disconnect();
			mounted = false;
		};
	});
</script>

<div class="lazyList">
	<button class="arrow-up" class:showJumpStart on:click={() => jumpTo(ListFetchDir.Before)}>
		<div />
	</button>
	<div class="lazyListView" bind:this={pan} on:scroll={handle_scroll}>
		<div class="scrollPane" bind:this={scrollPane}>
			{#each elems as item, index (item)}
				<div class="lazyListElement" data-index={index}>
					<slot {item} />
				</div>
			{/each}
		</div>
		{#if elems.length === 0}
			<div class="filler">
				{#if fetchTask !== undefined}
					<slot name="loading" />
				{:else}
					<slot name="empty" />
				{/if}
			</div>
		{/if}
	</div>
	<button class="arrow-down" class:showJumpEnd on:click={() => jumpTo(ListFetchDir.After)}>
		<div />
	</button>
</div>

<style lang="scss">
	.lazyList {
		position: relative;
		overflow: hidden;
	}

	.lazyListView {
		// The position: relative makes that getting offsetTop from child elements
		// will use the scrollPane element as the relative parent.
		position: relative;
		overflow-x: hidden;
		overflow-y: scroll;
		height: 100%;
	}

	// Jump start end buttons

	.arrow-down,
	.arrow-up {
		position: absolute;
		right: 2em;

		display: inline-block;
		background: #ccc;
		border-radius: 100%;
		padding: 0.8em;
		border: none;
		cursor: pointer;
		z-index: 3;

		transition-duration: 0.2s;
		transition-property: all;

		&:hover {
			background: #eee;
		}

		> div {
			border-left: 2px solid #222;
			border-top: 2px solid #222;
			width: 1em;
			height: 1em;
		}
	}

	.arrow-down {
		bottom: -5em;
		> div {
			transform: rotate(-135deg) translate(20%, 20%);
		}
	}

	.arrow-up {
		top: -5em;
		> div {
			transform: rotate(45deg) translate(20%, 20%);
		}
	}

	.showJumpStart {
		top: 1.5em;
	}

	.showJumpEnd {
		bottom: 1.5em;
	}

	// Anchoring
	// Shouldn't do much, but prevent our scrolling panel to be selected as
	// anchor since it's useless.
	.lazyListView,
	.scrollPane {
		overflow-anchor: none;
	}

	.filler {
		width: 100%;
		height: 100%;
	}
</style>
