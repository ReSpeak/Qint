<script lang="typescript">
	import { app } from "../../app";
	import BTabSlot from "../../ui/BTabSlot.svelte";
	import BKeyValue from "../../ui/BKeyValue.svelte";
	import BHotkeyField from "./BHotkeyField.svelte";
	import Icon from "../../ui/Icon.svelte";
	import { isHotkeyComplete } from "./hotkey";

	let localHotkeys = [...app.transientSettings.hotkeys.actions];
	function syncHotkeys() {
		app.transientSettings.hotkeys.actions = localHotkeys.filter(isHotkeyComplete);
		app.transientSettings.save();
		app.transientSettings.flush();
	}

	function deleteHotkey(index: number) {
		localHotkeys.splice(index, 1);
		localHotkeys = localHotkeys;
		syncHotkeys();
	}

	function createHotkey() {
		localHotkeys.push({
			keycode: null,
			_ctrl: false,
			_shift: false,
			_alt: false,
			_meta: false,
			action: null,
		});
		localHotkeys = localHotkeys;
	}
</script>

<BTabSlot title="Hotkeys">
	{#each localHotkeys as hotkey, index}
		<BHotkeyField
			{hotkey}
			on:change={() => syncHotkeys()}
			on:remove={() => deleteHotkey(index)} />
	{/each}

	<BKeyValue label="Add hotkey" labelStyle="is-normal">
		<button class="button" on:click={createHotkey}>
			<Icon name="plus" />
		</button>
	</BKeyValue>
</BTabSlot>
