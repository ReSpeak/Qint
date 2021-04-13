<script lang="typescript">
	import { createEventDispatcher, onDestroy, onMount } from "svelte";
	import {
		buildAction,
		getActionSubject,
		getActionValueFn,
		hotkeyToString,
		hotkeySubjects,
		hotkeyValueFns,
		translateJsKeyToWindows,
		isHotkeyComplete,
	} from "./hotkey";
	import type { Hotkey } from "../../transientSettings";
	import BDropDown from "../../ui/BDropDown.svelte";
	import Icon from "../../ui/Icon.svelte";
	import BKeyValue from "../../ui/BKeyValue.svelte";
	import debug from "debug";
	const log = debug("HOTKEY");

	export let hotkey: Hotkey;

	const dispatch = createEventDispatcher<{
		change: void;
		remove: void;
	}>();

	let input: HTMLInputElement;

	let selectedSubject = getActionSubject(hotkey.action);
	let selectedValueFn = getActionValueFn(hotkey.action);

	function onKeyHook(e: KeyboardEvent) {
		e.preventDefault();
		hotkey.keycode = translateJsKeyToWindows(e.code);
		hotkey._ctrl = e.ctrlKey;
		hotkey._shift = e.shiftKey;
		hotkey._alt = e.altKey;
		hotkey._meta = e.metaKey;

		input.value = hotkeyToString(hotkey);
		log("%j", hotkey);
		if (isHotkeyComplete(hotkey)) dispatch("change");
	}

	function onDropdownChange() {
		if (!selectedSubject || !selectedValueFn) return;
		hotkey.action = buildAction(selectedSubject, selectedValueFn);
		log("%j", hotkey);
		if (isHotkeyComplete(hotkey)) dispatch("change");
	}

	function onRemovePress(_: MouseEvent) {
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

<BKeyValue label="">
	<div class="is-horizontal field">
		<div class="control">
			<BDropDown
				on:change={onDropdownChange}
				items={hotkeySubjects}
				bind:selected={selectedSubject} />
		</div>
		<div class="control">
			<BDropDown
				on:change={onDropdownChange}
				items={hotkeyValueFns}
				bind:selected={selectedValueFn} />
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
</BKeyValue>
