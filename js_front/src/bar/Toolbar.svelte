<script lang="typescript">
	import { writable, Writable } from "svelte/store";
	import { Connection } from "../connection";
	import Icon from '../ui/Icon.svelte';

	export let connection!: Connection;
	export let showSidebar!: Writable<boolean>;
	export let showChat!: Writable<boolean>;
	let server = connection.book.server;
	let dropdownActive = false;
	let dropdown: HTMLElement;

	function handleFocus(event: FocusEvent) {
		// Check of the target lies within the dropdown
		if (event.relatedTarget && event.relatedTarget instanceof HTMLElement) {
			if (!dropdown.contains(event.relatedTarget)) {
				dropdownActive = false;
			}
		} else {
			dropdownActive = false;
		}
	}
</script>

<div class="toolbar">
	<div class="leftButtons">
		<button class="button toolbutton" class:active={$showSidebar} on:click={() => showSidebar.update(b => !b)}>
			<Icon name="file-tree" />
		</button>
		<button class="button toolbutton" class:active={$showChat} on:click={() => showChat.update(b => !b)}>
			<Icon name="chat-outline" />
		</button>
	</div>
	<div class="rightButtons">
		<div class="dropdown" bind:this={dropdown} class:is-active={dropdownActive} on:focusout={handleFocus}>
			<div class="dropdown-trigger">
				<button
					class="button toolbutton"
					aria-haspopup="true"
					aria-controls="dropdown-menu"
					on:click={() => dropdownActive = !dropdownActive}
				>
					<p class="image is-32x32">
						<img
							class="round"
							src="/128x128.png"
							alt="Home icon"
						/>
					</p>
				</button>
			</div>
			<div class="dropdown-menu" id="dropdown-menu3" role="menu" on:click={() => dropdownActive = false}>
				<div class="dropdown-content">
					<button class="button dropdown-item" on:click={() => connection.toggleMute()}>
						Toggle Mute
					</button>
					<button class="button dropdown-item">
						Settings
					</button>
					<hr class="dropdown-divider" />
					<button class="button dropdown-item" on:click={() => connection.disconnect()}>
						Disconnect
					</button>
				</div>
			</div>
		</div>
	</div>
</div>

<style lang="scss">
	.toolbar {
		background-color: $box-background-color;
		padding: 0.5em;
	}

	.leftButtons {
		float: left;
	}

	.rightButtons {
		float: right;
	}

	.toolbutton {
		background-color: #444444;
		border-radius: 100%;
	}
	.button {
		border: none;
		margin: 0.2em;
	}
	.button:focus {
		box-shadow: none;
	}

	.toolbutton.active {
		background-color: #888888;
	}

	.round {
		border-radius: 100%;
	}

	.dropdown-trigger button {
		margin-right: 0.5em;
		padding: 0;
		height: auto;
	}
</style>
