<script lang="typescript">
	import { onDestroy, onMount } from "svelte";
	import type { Hotkey } from "../hotkey";
	import { getActionState, actionToString, hotkeyToString } from "../hotkey";
	import BDropDown from "./BDropDown.svelte";
	import BKeyValue from "./BKeyValue.svelte";

	export let hotkey: Hotkey;
	
	let input: HTMLInputElement;
	let select: HTMLSelectElement;
	
	const selectOptions = [
		{ value: "True", text: "On" },
		{ value: "False", text: "Off" },
		{ value: "Toggle", text: "Toggle" },
	]

	function onKeyHook(e: KeyboardEvent) {
		e.preventDefault();
		let code = null;
		if (!["ControlLeft", "ControlRight", "ShiftLeft", "ShiftRight", "AltLeft", "AltRight", "MetaLeft", "MetaRight"].includes(e.code)) {
			code = translateJsKeyToWindows(e.code);
		}
		hotkey.keycode = code;
		hotkey.ctrl = e.ctrlKey;
		hotkey.shift = e.shiftKey;
		hotkey.alt = e.altKey;
		hotkey.meta = e.metaKey;

		input.value = hotkey.toString();
	}

	function translateJsKeyToWindows(jsKeyCode: string): string {
		// Too lazy to map the rest, have fun with this:
		// https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/code/code_values
		// https://github.com/LiveSplit/livesplit-core/blob/master/crates/livesplit-hotkey/src/windows/key_code.rs
		if (jsKeyCode.startsWith("Digit")) return jsKeyCode.replace("Digit", "D");
		if (jsKeyCode.startsWith("Key"))   return jsKeyCode.replace("Key", "");
		return jsKeyCode;
	}

	onMount(() => {
		console.log(hotkey);
		input?.addEventListener("focusin", () => {
			document.addEventListener("keydown", onKeyHook);
		});
		input?.addEventListener("focusout", () => {
			document.removeEventListener("keydown", onKeyHook);
		});
	});

	onDestroy(() => {
		document.removeEventListener("keydown", onKeyHook);
	});
</script>

<BKeyValue label={actionToString(hotkey.action)} labelStyle="is-normal">
	<div class="is-horizontal field">
		<div class="control">
			<BDropDown items={selectOptions} selected={getActionState(hotkey.action)}></BDropDown>
		</div>
		<div class="control">
			<input bind:this="{input}" class="input" value={hotkeyToString(hotkey)}>
		</div>
	</div>
</BKeyValue>