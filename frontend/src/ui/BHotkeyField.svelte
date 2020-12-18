<script lang="typescript">
	import { createEventDispatcher, onDestroy, onMount } from "svelte";
	import { actions, actionToName, valueToAction } from "../hotkey";
	import type { Hotkey } from "../hotkey";
	import { getActionState, hotkeyToString } from "../hotkey";
	import BDropDown from "./BDropDown.svelte";
	import Icon from "../ui/Icon.svelte";
	import BKeyValue from "./BKeyValue.svelte";

	export let hotkey: Hotkey;
	export let iconName: string;

	const dispatch = createEventDispatcher();
	
	let input: HTMLInputElement;
	
	let selectedAction = actionToName(hotkey.action);
	let selectedState = getActionState(hotkey.action);

	const stateOptions = [
		{ value: "", text: "" },
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

		dispatch("change", hotkey);
	}

	function onDropdownChange() {
		console.log(selectedAction);
		console.log(selectedState);
		if (!selectedAction || !selectedState) return;
		hotkey.action = valueToAction(selectedAction, selectedState);
		dispatch("change", hotkey);
	}

	function onButtonPress(_: MouseEvent) {
		dispatch("button", hotkey);
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

<BKeyValue label="">
	<div class="is-horizontal field">
		<div class="control">
			<BDropDown on:change={onDropdownChange} items={actions} bind:selected={selectedAction} />
		</div>
		<div class="control">
			<BDropDown on:change={onDropdownChange} items={stateOptions} bind:selected={selectedState} />
		</div>
		<div class="control">
			<input bind:this={input} class="input" value={hotkeyToString(hotkey)}>
		</div>
		<div class="control">
			<button on:click={onButtonPress} class="button">
				<Icon name={iconName} />
			</button>
		</div>
	</div>
</BKeyValue>