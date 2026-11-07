<script lang="ts">
	import { app } from "../app";
	import { ChatMode } from "../settings";
	import { matrixStatus } from "../matrix";
	import { showContextMenu } from "../contextMenu";
	import Icon from "../ui/icon/Icon.svelte";

	const chatMode = app.settings.chat._chatMode;
	const modes = [
		{ mode: ChatMode.TeamSpeak, label: "TeamSpeak chat" },
		{ mode: ChatMode.Matrix, label: "Matrix chat" },
	];
	let open = false;

	function toggle(ev: MouseEvent) {
		if (open) return;
		// Reuse the context menu handling to close on outside click/Escape
		showContextMenu(ev, () => (open = false));
		open = true;
	}

	function select(mode: ChatMode) {
		$chatMode = mode;
		app.settings.save();
	}
</script>

{#if $matrixStatus.loggedIn}
	<div class="dropdown is-up is-right" class:is-active={open}>
		<div class="dropdown-trigger">
			<button
				class="button outline-button"
				type="button"
				title="Switch chat"
				aria-haspopup="true"
				on:click={toggle}
			>
				<Icon name="menu-up" />
			</button>
		</div>
		<div class="dropdown-menu" role="menu">
			<div class="dropdown-content">
				{#each modes as m}
					<a
						href={"#"}
						class="dropdown-item"
						class:is-active={$chatMode === m.mode}
						on:click|preventDefault={() => select(m.mode)}>{m.label}</a
					>
				{/each}
			</div>
		</div>
	</div>
{/if}

<style>
	.dropdown,
	.dropdown-trigger,
	.dropdown-trigger .button {
		height: 100%;
	}
</style>
