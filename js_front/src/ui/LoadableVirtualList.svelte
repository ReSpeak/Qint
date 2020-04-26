<script>
	// Based on https://github.com/sveltejs/svelte-virtual-list/blob/master/VirtualList.svelte
	import { afterUpdate, onMount, tick } from 'svelte';

	// props
	// `true` if the view starts at the top and scrolls to the bottom,
	// `false` if the view starts at the bottom.
	export let startIsTop = true;
	// A function that takes a boolean, `true` if we need more data at the start,
	// `false` if we need more data at the end.
	export let loadMore;
	export let maxItems = 500; // Don't crash the browser if something goes havoc

	// read-only, but visible to consumers via bind:items
	export let items = [];

	// local state
	let rows;
	let viewport;
	let contents;
	let viewport_height = 0;
	let mounted;
	// `true` if `items` is at the start and no more items can be loaded.
	export let isAtStart = true;
	// `true` if `items` is at the end and no more items can be loaded.
	export let isAtEnd = false;

	// The promise for loading items
	let loadingStart;
	let loadingEnd;
	let isLoading = false;
	let arrowHidden = true;
	let lastScrollTop = 0;

	let top = 0;
	let bottom = 0;

	// Refresh if something changes
	$: if (mounted) loadData(viewport_height);

	export function update() {
		return handle_scroll();
	}

	export function clear() {
		items = [];
		isAtStart = startIsTop;
		isAtEnd = !startIsTop;
		// TODO Cancel running promises
		loadingStart = undefined;
		loadingEnd = undefined;
		arrowHidden = true;
		lastScrollTop = 0;
		top = 0;
		bottom = 0;
		update();
	}

	// There are new items available
	export async function newItems(atStart) {
		// If the view is fully scrolled to the end and we need to scroll after updating
		let awayFromBorder;
		if (atStart && isAtStart) {
			isAtStart = false;
			awayFromBorder = viewport.scrollTop == 0;
		} else if (!atStart && isAtEnd) {
			isAtEnd = false;
			awayFromBorder = viewport.scrollHeight - viewport.clientHeight - viewport.scrollTop;
		} else {
			// Nothing to do
			return;
		}
		if (awayFromBorder > viewport.clientHeight * 2) {
			return;
		}

		await loadData();
		if (awayFromBorder == 0) {
			if (atStart)
				viewport.scrollTo(0, 0);
			else
				viewport.scrollTo(0, viewport.scrollHeight - viewport.clientHeight);
		}
	}

	async function loadData() {
		// Already loading
		if (isLoading)
			return;
		isLoading = true;
		await tick(); // wait until the DOM is up to date
		if (!viewport)
			return;

		// Load as long as necessary
		while (true) {
			if (items.length >= maxItems) {
				isLoading = false;
				return;
			}

			// Invisible content around the viewport
			const content_buffer_start = viewport.scrollTop - top;
			const content_buffer_end = Math.max(0, viewport.scrollHeight - viewport.scrollTop - viewport.clientHeight - bottom);

			// Check if we need to load more
			if (!isAtStart && content_buffer_start < viewport.clientHeight) {
				loadingStart = loadMore(true).finally(() => loadingStart = undefined);
				const newItems = await loadingStart;
				if (newItems) {
					items = [...newItems, ...items];
					const oldHeight = contents.offsetHeight;
					console.log("0", viewport.scrollTop, viewport.scrollHeight, viewport.clientHeight, oldHeight);

					// Prevent jumping
					await tick();

					let new_height = contents.offsetHeight - oldHeight;
					console.log("1", viewport.scrollTop, viewport.scrollHeight, viewport.clientHeight);

					let rest = 0;
					if (new_height > top)
						rest = new_height - top;
					top = Math.max(0, top - new_height);
					// TODO Stutters here sometimes because we interupt the smooth scrolling
					viewport.scrollTo(0, viewport.scrollTop + rest);
					console.log("2", viewport.scrollTop, viewport.scrollHeight, viewport.clientHeight);
				} else {
					isAtStart = true;
					top = 0;
				}
			} else if (!isAtEnd && content_buffer_end < viewport.clientHeight) {
				loadingEnd = loadMore(false).finally(() => loadingEnd = undefined);
				const newItems = await loadingEnd;
				if (newItems) {
					items = [...items, ...newItems];

					// Wait until loaded
					await tick();

					let new_height = 0;
					for (let i = items.length - newItems.length; i < items.length; i++)
						new_height += rows[i].offsetHeight;

					let rest = 0;
					if (new_height > bottom)
						rest = new_height - bottom;
					bottom = Math.max(0, bottom - new_height);
				} else {
					isAtEnd = true;
					bottom = 0;
				}
			} else {
				isLoading = false;
				return;
			}
		}
		isLoading = false;
	}

	function getRowTop(i) {
		let t = -1;
		const cs = rows[i].children;
		for (let j = 0; j < cs.length; j++) {
			const c = cs[j];
			if (t == -1 || (c.offsetTop != 0 && c.offsetTop < t)) {
				t = c.offsetTop;
			}
		}
		return t;
	}

	async function handle_scroll() {
		if (!viewport)
			return;
		// Show or hide return button
		if ((viewport.scrollTop < lastScrollTop) == startIsTop
			&& viewport.scrollTop != 0 && viewport.scrollHeight - viewport.scrollTop != viewport.clientHeight) {
			arrowHidden = false;
		} else {
			arrowHidden = true;
		}
		lastScrollTop = viewport.scrollTop;

		// Already loading, update nothing
		if (isLoading)
			return;

		await loadData();

		if (rows.length != items.length) {
			console.error("Should have the same amount of rows as items", rows, items);
			return;
		}

		// Remove excessive items
		let i = 0;
		let y = 0;
		const constOffset = rows.length > 0 ? getRowTop(0) : 0;
		const maxTop = viewport.scrollTop - constOffset - 2 * viewport.clientHeight;

		// Could be binary search
		while (i < rows.length) {
			if (getRowTop(i) >= maxTop)
				break;
			i += 1;
		}

		const start = i;
		if (start == rows.length) {
			if (start != 0)
				clear();
			return;
		}

		top = getRowTop(start) - constOffset;

		const minTop = viewport.scrollTop + 3 * viewport.clientHeight;
		while (i < rows.length) {
			if (getRowTop(i) > minTop)
				break;
			i += 1;
		}

		const end = i;
		let rest = 0;
		if (rows.length > 0 && end < rows.length) {
			const oldEnd = getRowTop(rows.length - 1) + rows[rows.length - 1].offsetHeight;
			const newEnd = getRowTop(end) + rows[end].offsetHeight
			rest = oldEnd - newEnd;
		}

		// Clamp top and bottom
		const maxSize = 50 * viewport.clientHeight;
		bottom = Math.min(bottom + rest, maxSize);
		if (top > maxSize) {
			rest = top - maxSize;
			top = maxSize;
			viewport.scrollTo(0, viewport.scrollTop - rest);
		}

		if (start > 0)
			isAtStart = false;
		if (end < rows.length)
			isAtEnd = false;
		if (start != 0 || end != rows.length) {
			console.log("slice", start, end, rows.length, top, bottom);
			items = items.slice(start, end);
		}
	}

	function scrollReturn() {
		if (startIsTop) {
			if (isAtStart)
				viewport.scrollTo(0, 0);
			else
				clear();
		} else {
			if (isAtEnd)
				viewport.scrollTo(0, viewport.scrollHeight - viewport.clientHeight);
			else
				clear();
		}
	}

	// trigger initial refresh
	onMount(() => {
		rows = contents.children;
		isAtStart = startIsTop;
		isAtEnd = !startIsTop;
		mounted = true;
	});
</script>

<svelte-virtual-list>
{#if !startIsTop}
	<slot name="returnArrow"><button class="arrow-down" class:arrowHidden on:click={scrollReturn}><div></div></button></slot>
{/if}
<svelte-virtual-list-viewport
	bind:this={viewport}
	bind:offsetHeight={viewport_height}
	on:scroll={handle_scroll}
>
	{#if loadingStart}
		<slot name="loading"><div>Loading…</div></slot>
	{/if}
	<svelte-virtual-list-contents
		bind:this={contents}
		style="padding-top: {top}px; padding-bottom: {bottom}px;"
	>
		{#each items as item}
			<svelte-virtual-list-item>
				<slot {item}>Missing template</slot>
			</svelte-virtual-list-item>
		{/each}
	</svelte-virtual-list-contents>
	{#if loadingEnd}
		<slot name="loading"><div>Loading…</div></slot>
	{/if}
</svelte-virtual-list-viewport>
{#if startIsTop}
	<slot name="returnArrow"><button class="arrow-up" class:arrowHidden on:click={scrollReturn}><div></div></button></slot>
{/if}
</svelte-virtual-list>

<style>
	svelte-virtual-list {
		display: block;
		position: relative;
		overflow-y: hidden;
	}

	svelte-virtual-list-viewport {
		display: grid;
		align-items: end;
		position: relative;
		overflow-y: auto;
	}

	svelte-virtual-list-viewport :global(object) {
		display: none !important;
	}

	svelte-virtual-list-contents {
		display: table-cell;
	}

	svelte-virtual-list-item {
		display: contents;
	}

	.arrow-down, .arrow-up {
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

	.arrow-down:hover, .arrow-up:hover {
		background: #eee;
	}

	.arrowHidden {
		bottom: -5em;
	}

	.arrow-down > div, .arrow-up > div {
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