<script lang="typescript">
	import { onMount } from "svelte";
	import { getResizeObserver } from "../util";
	let stickyList!: HTMLElement;
	let stickyChildren: ArrayLike<HTMLElement> = [];
	let stickySizes = [] as number[];
	let stickyAcc = [] as number[];

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

	onMount(() => {
		stickyChildren = stickyList.querySelectorAll<HTMLElement>(":scope > .stickySlot");
		stickySizes = Array(stickyChildren.length);
		stickyAcc = Array(stickyChildren.length);
		let obs = getResizeObserver(() => updateChildSize());
		for (let i = 0; i < stickyChildren.length; i++) {
			stickyChildren[i].onclick = () => {
				let nextElement = stickyChildren[i].nextElementSibling! as HTMLElement;
				stickyList.scrollTop = nextElement.offsetTop - stickyList.offsetTop - stickyAcc[i];
			}
			stickySizes[i] = stickyChildren[i].offsetHeight;
			obs.observe(stickyChildren[i]);
		}

		updateChildSize();
	});
</script>

<div bind:this={stickyList} class="stickyList">
	<slot />
</div>

<style lang="scss">
	.stickyList {
		display: flex;
		flex-direction: column;
		height: 100%;
		overflow-x: hidden;
		overflow-y: auto;
	}
</style>
