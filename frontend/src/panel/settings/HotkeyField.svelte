<script lang="ts">
	import { createEventDispatcher, onDestroy, onMount } from "svelte";
	import {
		buildAction,
		getActionSubject,
		hotkeyToString,
		hotkeySubjects,
		translateJsKeyToWindows,
		isHotkeyComplete,
	} from "./hotkey";
	import type { Hotkey } from "../../transientSettings";
	import DropDown from "../../ui/html/DropDown.svelte";
	import Icon from "../../ui/icon/Icon.svelte";
	import KeyValue from "../../ui/util/KeyValue.svelte";
	import debug from "debug";
	const log = debug("HOTKEY");

	export let hotkey: Hotkey;

	const dispatch = createEventDispatcher<{
		change: void;
		remove: void;
	}>();

	let input: HTMLInputElement;

	let selectedSubject = getActionSubject(hotkey.action);

	function onKeyHook(e: KeyboardEvent) {
		e.preventDefault();
		log("RAW: code:%j char:%j charCode:%j", e.code, e.char, e.charCode);
		hotkey.keycode = translateJsKeyToWindows(e.code);
		hotkey._ctrl = e.ctrlKey;
		hotkey._shift = e.shiftKey;
		hotkey._alt = e.altKey;
		hotkey._meta = e.metaKey;

		input.value = hotkeyToString(hotkey);
		log("KeyData %j", hotkey);
		if (isHotkeyComplete(hotkey)) dispatch("change");
	}

	function onDropdownChange() {
		if (!selectedSubject) return;
		hotkey.action = buildAction(selectedSubject);
		log("%j", hotkey);
		if (isHotkeyComplete(hotkey)) dispatch("change");
	}

	function onRemovePress() {
		dispatch("remove");
	}

	onMount(() => {
		input.addEventListener("focusin", () => {
			document.addEventListener("keydown", onKeyHook);
		});
		input.addEventListener("focusout", () => {
			document.removeEventListener("keydown", onKeyHook);
		});
	});

	onDestroy(() => {
		document.removeEventListener("keydown", onKeyHook);
	});
</script>

<KeyValue label="">
	<div class="is-horizontal field">
		<div class="control">
			<DropDown
				on:change={onDropdownChange}
				items={hotkeySubjects}
				bind:selected={selectedSubject} />
		</div>
		<div class="control">
			<input bind:this={input} class="input" value={hotkeyToString(hotkey)} />
		</div>
		<div class="control">
			<button on:click={onRemovePress} class="button">
				<Icon name="close" />
			</button>
		</div>
	</div>
</KeyValue>
