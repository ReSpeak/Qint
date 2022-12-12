<script lang="ts">
	import { app } from "../../app";
	import TabSlot from "../../ui/container/TabSlot.svelte";
	import KeyValue from "../../ui/util/KeyValue.svelte";
	import HotkeyField from "./HotkeyField.svelte";
	import Icon from "../../ui/icon/Icon.svelte";
	import { isHotkeyComplete } from "./hotkey";

	let localHotkeys = [...app.settings.hotkeys.actions];
	function syncHotkeys() {
		app.settings.hotkeys.actions = localHotkeys.filter(isHotkeyComplete);
		app.settings.save();
		app.settings.flush();
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

<TabSlot title="Hotkeys">
	{#each localHotkeys as hotkey, index}
		<HotkeyField
			{hotkey}
			on:change={() => syncHotkeys()}
			on:remove={() => deleteHotkey(index)}
		/>
	{/each}

	<KeyValue label="Add hotkey" labelStyle="is-normal">
		<button class="button" on:click={createHotkey}>
			<Icon name="plus" />
		</button>
	</KeyValue>
</TabSlot>
