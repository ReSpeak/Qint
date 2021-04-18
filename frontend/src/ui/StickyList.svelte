<script lang="ts">
	import { onDestroy } from "svelte";
	import ResizeObserver from 'resize-observer-polyfill';
	import { setContext } from 'svelte';

	let stickyList: HTMLElement;
	let stickyChildren: ArrayLike<HTMLElement> = [];
	let stickySizes = [] as number[];
	let stickyAcc = [] as number[];
	let obs: ResizeObserver | undefined = undefined;

	function updateChildSize() {
		let topOff = 0;
		for (let i = 0; i < stickyChildren.length; i++) {
			stickySizes[i] = stickyChildren[i].offsetHeight;
			stickyChildren[i].style.top = `${topOff}px`;
			topOff += stickySizes[i];
			stickyAcc[i] = topOff;
		}
		let botOff = 0;
		for (let i = stickyChildren.length - 1; i >= 0; i--) {
			stickyChildren[i].style.bottom = `${botOff}px`;
			botOff += stickySizes[i];
		}
	}

	function stickyChanged() {
		if (!stickyList) return;
		stickyChildren = Array(...stickyList.children).filter(c => c.matches(".stickySlot")) as HTMLElement[];
		stickySizes = Array(stickyChildren.length);
		stickyAcc = Array(stickyChildren.length);
		obs?.disconnect();
		obs = new ResizeObserver(() => updateChildSize());
		for (let i = 0; i < stickyChildren.length; i++) {
			stickyChildren[i].onclick = () => {
				const nextElement = stickyChildren[i].nextElementSibling! as HTMLElement;
				stickyList.scrollTop = nextElement.offsetTop - stickyList.offsetTop - stickyAcc[i];
			}
			stickySizes[i] = stickyChildren[i].offsetHeight;
			obs.observe(stickyChildren[i]);
		}

		updateChildSize();
	}

	setContext('stickyChanged', stickyChanged);

	onDestroy(() => { obs?.disconnect(); });
	//onMount();
</script>

<div bind:this={stickyList} class="stickyList">
	<slot />
	<div class="stickEndDummy"></div>
</div>

<style lang="scss">
	.stickyList {
		display: flex;
		flex-direction: column;
		height: 100%;
		overflow-x: hidden;
		overflow-y: auto;
	}

	.stickEndDummy {
		min-height: 1px;
	}
</style>
