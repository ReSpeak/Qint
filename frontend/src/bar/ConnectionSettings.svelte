<script lang="typescript">
	import Icon from "../ui/Icon.svelte";
	import type { Writable } from "svelte/store";
	import { Client } from "../book";
	import type { OChangeConnectionClientUpdate } from "../book_events";
	import { Connection } from "../connection";

	export let connection: Connection;

	let inputMuted = false;
	let outputMuted = false;
	let isAway = false;

	let ownClient: Writable<Client | undefined> | undefined;
	$: {
		ownClient = connection.book.ownClient;
		inputMuted = $ownClient?.inputMuted ?? false;
		outputMuted = $ownClient?.outputMuted ?? false;
		const awayMessage = $ownClient?.awayMessage;
		isAway = awayMessage !== undefined && awayMessage !== null;
	}

	function changeOwnClient(change: OChangeConnectionClientUpdate["ConnectionClientUpdate"]) {
		connection.sendMessage({
			Change: {
				change: { ConnectionClientUpdate: change },
			},
		});
	}
</script>

<div class="toolbuttons">
	<button
		class="toolbutton"
		class:active={inputMuted}
		on:click={() => changeOwnClient({ inputMuted: !inputMuted })}
		title="Mute microphone">
		<Icon name={inputMuted ? 'microphone-off' : 'microphone'} />
	</button>
	<button
		class="toolbutton"
		class:active={outputMuted}
		on:click={() => changeOwnClient({ outputMuted: !outputMuted })}
		title="Mute output">
		<Icon name={outputMuted ? 'volume-off' : 'volume-high'} />
	</button>
	<button
		class="toolbutton"
		class:active={isAway}
		on:click={() => changeOwnClient({ away: isAway ? null : '' })}
		title="Toggle away">
		<Icon name={isAway ? 'sleep' : 'sleep-off'} />
	</button>
</div>

<style lang="scss">
</style>
