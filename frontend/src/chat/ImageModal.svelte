<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import { app } from "../app";

	export let src: string;
	export let visible: boolean;
	let div: HTMLElement;
	let img: HTMLImageElement;
	let stretch = false;

	$: app.modalVisible.set(visible);

	function updateStretch() {
		stretch = img.naturalWidth === 0 && img.naturalHeight === 0;
	}

	onMount(() => {
		updateStretch();
		div.focus();
	});
	onDestroy(() => app.modalVisible.set(false));
</script>

<!-- Tabindex to make the div focusable and trigger onkeydown -->
<div
	bind:this={div}
	class="modal is-active"
	on:click={() => (visible = false)}
	on:keydown={(e) => {
		if (e.key === "Escape") {
			e.stopPropagation();
			visible = false;
		}
	}}
	tabindex="0">
	<div class="modal-background" />
	<!-- svelte-ignore a11y-missing-attribute -->
	<div class="custom-content" class:stretch>
		<img bind:this={img} on:load={() => updateStretch()} {src} />
	</div>
	<button class="modal-close is-large" aria-label="close" />
</div>

<style lang="scss">
	img {
		max-height: 100%;
		max-width: 100%;
	}

	$max: calc(100% - 2em);

	.custom-content {
		position: relative;
		max-width: $max;
		max-height: $max;
		display: flex;
		justify-content: center;
	}

	.stretch {
		min-width: $max;
		min-height: $max;
	}

	.modal {
		position: fixed;
	}
</style>
