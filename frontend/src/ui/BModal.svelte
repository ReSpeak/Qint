<script lang="typescript">
    import { app } from "../app";

    export let visible: boolean;
	export let title: string;
	
	function close(e: Event) {
		visible = false;
		e.stopPropagation();
	}

    $: {
		app.modalVisible.set(visible);
    }
</script>

<div class="modal" class:is-active={visible}>
	<div on:click={close} class="modal-background"></div>
	<div class="modal-card">
		<header class="modal-card-head">
			<p class="modal-card-title">{title}</p>
			<slot name="header" />
		</header>
		<section class="modal-card-body">
			<slot name="content" />
		</section>
		<footer class="modal-card-foot">
			<slot name="footer" />
		</footer>
	</div>
	<button on:click={close} class="modal-close is-large" aria-label="close"></button>
</div>

<style lang="scss">
	@import "bulma/sass/components/modal";
	@import "bulmaswatch/cyborg/overrides";
</style>