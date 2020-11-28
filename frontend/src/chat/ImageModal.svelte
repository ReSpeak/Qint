<script lang="typescript">
	import { onDestroy } from "svelte";
	import { app } from "../app";

	export let src: string;
	export let visible: boolean;

	$: app.modalVisible.set(visible);

	onDestroy(() => app.modalVisible.set(false));

	function sroll() {
		console.log("asdfkajlsdfkjhasdf");
	}
</script>

<div class="modal is-active" on:click={() => (visible = false)}>
	<div class="modal-background" />
	<!-- svelte-ignore a11y-missing-attribute -->
	<div class="custom-content"><img {src} /></div>
	<button class="modal-close is-large" aria-label="close" />
</div>

<style lang="scss">
	img {
		max-height: 100%;
		max-width: 100%;
	}

	.custom-content {
		position: relative;
		max-width: calc(100% - 2em);
		max-height: calc(100% - 2em);
		display: flex;
		justify-content: center;
	}

	.modal {
		position: fixed;
	}
</style>
