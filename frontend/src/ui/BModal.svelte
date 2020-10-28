<script lang="typescript">
	import { app } from "../app";

	export let visible: boolean;
	export let title: string = "";

	function close(e: Event) {
		visible = false;
		e.stopPropagation();
	}

	function keydown(e: KeyboardEvent) {
		if (e.key === "Escape")
			close(e);
	}

	$: {
		app.modalVisible.set(visible);
	}
</script>

<div on:keydown={keydown} class="modal" class:is-active={visible}>
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
	<button on:click={close} class="modal-close is-large" aria-label="close" />
</div>

<style lang="scss">
	@import "bulma/sass/components/modal";
	@import "bulmaswatch/cyborg/overrides";

	.modal {
		z-index: 200;
	}
</style>
