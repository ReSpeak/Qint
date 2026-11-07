<script lang="ts">
	// Mounts the real LazyList with a mocked chat source. Content with
	// late-known height (images, link previews) is simulated via ?delay=ms.
	import LazyList from "../src/ui/container/LazyList.svelte";
	import { ListFetchDir } from "../src/ui/container/uiLazyList";
	import type { FetchResult } from "../src/ui/container/uiLazyList";

	type Item = { id: number; text: string; img?: { h: number; delay: number }; preview?: number };

	const params = new URLSearchParams(location.search);
	const total = Number(params.get("total") ?? 300);
	const pageSize = Number(params.get("page") ?? 50);
	const maxDelay = Number(params.get("delay") ?? 800);
	const fetchDelay = Number(params.get("fetch") ?? 30);

	// Deterministic PRNG so runs are reproducible per seed/channel
	function rng(seed: number) {
		return () => ((seed = (seed * 1103515245 + 12345) % 2 ** 31) / 2 ** 31);
	}

	function makeChannel(seed: number): Item[] {
		const r = rng(seed);
		return Array.from({ length: total }, (_, id) => ({
			id,
			text: `ch${seed} msg ${id} ` + "lorem ipsum ".repeat(1 + Math.floor(r() * 10)),
			img: r() < 0.25 ? { h: 100 + Math.floor(r() * 300), delay: Math.floor(r() * maxDelay) } : undefined,
			preview: r() < 0.15 ? Math.floor(r() * maxDelay) : undefined,
		}));
	}

	let channel = 1;
	let data = makeChannel(channel);
	let list: LazyList<Item>;

	async function fetchElements(from: Item | undefined, dir: ListFetchDir): Promise<FetchResult<Item>> {
		await new Promise((res) => setTimeout(res, fetchDelay));
		let start: number, end: number;
		if (dir === ListFetchDir.New) [start, end] = [total - pageSize, total];
		else if (dir === ListFetchDir.Before) [start, end] = [from!.id - pageSize, from!.id];
		else [start, end] = [from!.id + 1, from!.id + 1 + pageSize];
		start = Math.max(0, start);
		end = Math.min(total, end);
		return { items: data.slice(start, end), canLoadBeforeStart: start > 0, canLoadAfterEnd: end < total };
	}

	export function switchChannel(ch: number) {
		channel = ch;
		data = makeChannel(ch);
		list.sourceChanged(ListFetchDir.New, ListFetchDir.After);
	}
	(window as any).harness = { switchChannel };

	$: if (list) list.sourceChanged(ListFetchDir.New, ListFetchDir.After);

	// Svelte action: render content only after `delay` ms (unknown height until then)
	function late(node: HTMLElement, delay: number) {
		node.style.display = "none";
		setTimeout(() => (node.style.display = ""), delay);
	}
</script>

<div class="chat">
	<LazyList bind:this={list} {fetchElements} suggestJumpEnd={true} let:item>
		<div slot="loading">Loading ...</div>
		<div slot="empty">empty</div>
		<div class="msg">{item.text}</div>
		{#if item.img}
			<div use:late={item.img.delay} class="img" style="height:{item.img.h}px">img {item.img.h}px</div>
		{/if}
		{#if item.preview !== undefined}
			<div use:late={item.preview} class="preview">link preview<br />title<br />description</div>
		{/if}
	</LazyList>
</div>

<style>
	:global(body) {
		margin: 0;
		font-family: sans-serif;
	}
	.chat {
		height: 600px;
		width: 500px;
		display: flex;
		flex-direction: column;
		justify-content: flex-end;
		> :global(.lazyList) {
			flex: 1;
		}
	}
	.msg {
		padding: 2px 4px;
	}
	.img {
		background: #8ac;
		width: 200px;
	}
	.preview {
		border: 1px solid #888;
		margin: 4px;
		padding: 4px;
	}
</style>
