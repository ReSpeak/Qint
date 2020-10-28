<script lang="typescript">
    import { app } from "../app";

    export let visible: boolean;
    export let name: string;
	
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
	<div class="modal-content">
		lul
		<slot />
	</div>
	<button on:click={close} class="modal-close is-large" aria-label="close"></button>
</div>

<style lang="scss">
    @import "bulma/sass/components/modal";
</style>