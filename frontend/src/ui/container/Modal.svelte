<script lang="ts">
	import { onDestroy } from "svelte";
	import { app } from "../../app";

	export let visible: boolean;
	export let title: string = "";

	function close(e: Event) {
		visible = false;
		e.stopPropagation();
	}

	$: app.modalVisible.set(visible);

	onDestroy(() => app.modalVisible.set(false));
</script>

<!-- Tabindex to make the div focusable and trigger onkeydown -->
<div
	on:keydown={(e) => {
		if (e.key === "Escape") close(e);
	}}
	tabindex="0"
	class="modal"
	class:is-active={visible}>
	<div on:click={close} class="modal-background" />
	<div class="modal-card">
		<header class="modal-card-head">
			<p class="modal-card-title">
				<slot name="header">{title}</slot>
			</p>
		</header>
		<section class="modal-card-body">
			<slot />
		</section>
		<footer class="modal-card-foot">
			<slot name="footer" />
		</footer>
	</div>
	<button on:click|preventDefault={close} class="modal-close is-large" aria-label="close" />
</div>
