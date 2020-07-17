<script lang="typescript">
	import { onMount } from "svelte";
	export let value!: string;
	let setValue: string | undefined;
	let self!: HTMLElement;

	$: if (value !== setValue) {
		applyValue(value);
	}

	export function focus() {
		self.focus();
	}

	function textChanged() {
		let tmp = self.innerText;
		if (tmp.endsWith("\n")) tmp = tmp.substring(0, tmp.length - 1);
		setValue = tmp;
		value = tmp;
	}

	function applyValue(val: string) {
		if (val !== setValue) {
			setValue = val;
			if (self) self.innerText = val;
		}
	}

	onMount(() => {
		setValue = undefined;
		applyValue(value);
	});
</script>

<div
	bind:this={self}
	on:keydown
	on:input={textChanged}
	class="input chatTextBox"
	name="message"
	contenteditable="true" />

<style>
	.chatTextBox {
		-moz-appearance: textfield;
		-webkit-appearance: textfield;
		height: auto;
		display: block;
		white-space: pre-wrap;
	}
</style>
