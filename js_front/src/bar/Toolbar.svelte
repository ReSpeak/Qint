<script lang="typescript">
	import { writable, Writable } from "svelte/store";
	import { Connection } from "../connection";
	import Icon from '../ui/Icon.svelte';

	export let connection!: Connection;
	export let showSidebar!: boolean;
	export let showChat!: boolean;
	export let showGlobalSettings!: boolean;
	let server = connection.book.server;
	let dropdownActive = false;
	let dropdown: HTMLElement;

	let ownClient = connection.ownClient;
	declare let input_muted: boolean | undefined;
	$: input_muted = $ownClient?.input_muted;
	declare let output_muted: boolean | undefined;
	$: output_muted = $ownClient?.output_muted;
	declare let is_away: boolean | undefined;
	$: is_away = $ownClient?.away_message !== null;

	function changeOwnClient(change: any) {
		connection.sendMessage({
			Events: [{
				PropertyChanged: {
					id: {
						Client: connection.ownClientId!,
					},
					prop: { Client: change },
					invoker: null,
					extra: { reason: null },
				}
			}]
		});
	}

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
		<button class="button toolbutton" class:active={showSidebar} on:click={() => showSidebar = !showSidebar}>
			<Icon name="file-tree" />
		</button>
		<button class="button toolbutton" class:active={showChat} on:click={() => showChat = !showChat}>
			<Icon name="chat-outline" />
		</button>
	</div>
	<div class="rightButtons">
		<button class="button toolbutton" class:active={input_muted} on:click={() => changeOwnClient({ input_muted: !input_muted })}>
			<Icon name={input_muted ? "microphone-off" : "microphone"} />
		</button>
		<button class="button toolbutton" class:active={output_muted} on:click={() => changeOwnClient({ output_muted: !output_muted })}>
			<Icon name={output_muted ? "volume-off" : "volume-high"} />
		</button>
		<button class="button toolbutton" class:active={is_away} on:click={() => changeOwnClient({ away_message: is_away ? null : "" })}>
			<Icon name={is_away ? "sleep" : "sleep-off"} />
		</button>

		<div class="dropdown is-right" bind:this={dropdown} class:is-active={dropdownActive} on:focusout={handleFocus}>
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
					<button class="button dropdown-item" on:click={() => showGlobalSettings = true}>
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

	.button.toolbutton {
		background-color: #444444;
		border-radius: 100%;
	}
	.button {
		background: none;
		border: none;
		margin: 0.2em;
	}
	.button.dropdown-item:hover {
		background: none;
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
