<script lang="ts">
	import Icon from "../ui/icon/Icon.svelte";
	import type { Writable } from "svelte/store";
	import { Client } from "../book";
	import type { OChangeConnectionClientUpdate } from "../book_events";
	import { Connection } from "../connection";
	import { ConnectData, MuteState } from "../connect/uiConnect";
	import { backend } from "../backend/backend";
	import { app } from "../app";

	export let connection: Connection | undefined = undefined;
	export let connectData: ConnectData | undefined = undefined;

	let inputMuted = false;
	let outputMuted = false;
	let isAway = false;

	let ownClient: Writable<Client | undefined> | undefined;
	$: {
		if (connection !== undefined) {
			ownClient = connection.book.ownClient;
			inputMuted =
				($ownClient?.inputMuted ?? false) || !($ownClient?.inputHardwareEnabled ?? true);
			outputMuted =
				($ownClient?.outputMuted ?? false) || !($ownClient?.outputHardwareEnabled ?? true);
			const awayMessage = $ownClient?.awayMessage;
			isAway = awayMessage !== undefined && awayMessage !== null;
		} else if (connectData !== undefined) {
			inputMuted = connectData.inputMuted !== MuteState.None;
			outputMuted = connectData.outputMuted !== MuteState.None;
			isAway = connectData.away !== undefined;
		}
	}

	async function changeOwnClient(
		change: OChangeConnectionClientUpdate["ConnectionClientUpdate"]
	) {
		if (change.inputMuted !== undefined) {
			inputMuted = change.inputMuted;
			if (connectData !== undefined)
				connectData.inputMuted = inputMuted ? MuteState.Muted : MuteState.None;
		}
		if (change.outputMuted !== undefined) {
			outputMuted = change.outputMuted;
			if (connectData !== undefined)
				connectData.outputMuted = outputMuted ? MuteState.Muted : MuteState.None;
		}
		if (change.away !== undefined) {
			isAway = change.away !== null;
			if (connectData !== undefined) connectData.away = isAway ? "" : undefined;
		}

		await connection?.sendChange({
			ConnectionClientUpdate: change,
		});

		if (connection === undefined) {
			// Send as shortcut
			if (change.inputMuted !== undefined) {
				await backend.run_hotkey({ InputMute: null });
			}
			if (change.outputMuted !== undefined) {
				await backend.run_hotkey({ OutputMute: null });
			}
			if (change.away !== undefined) {
				await backend.run_hotkey({ Away: null });
			}
		}
		app.updateMuteState();
	}
</script>

<div class="toolbuttons">
	<button
		class="toolbutton"
		class:active={inputMuted}
		on:click={() => changeOwnClient({ inputMuted: !inputMuted })}
		title="Mute microphone">
		<Icon name={inputMuted ? "microphone-off" : "microphone"} />
	</button>
	<button
		class="toolbutton"
		class:active={outputMuted}
		on:click={() => changeOwnClient({ outputMuted: !outputMuted })}
		title="Mute output">
		<Icon name={outputMuted ? "volume-off" : "volume-high"} />
	</button>
	<button
		class="toolbutton"
		class:active={isAway}
		on:click={() => changeOwnClient({ away: isAway ? null : "" })}
		title="Toggle away">
		<Icon name={isAway ? "sleep" : "sleep-off"} />
	</button>
</div>
