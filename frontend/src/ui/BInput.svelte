<script lang="typescript">
	import { createEventDispatcher, onMount, tick } from "svelte";
	export let value: string;
	export let enterToSubmit = true;
	const submitDispatch = createEventDispatcher<{ submit: undefined }>();
	let setValue: string | undefined;
	let self!: HTMLElement;
	let expectQuickPaste = false;

	$: if (value !== setValue) {
		applyValue(value);
	}

	export async function focus() {
		const range = document.createRange();
		range.selectNodeContents(self);
		const sel = window.getSelection()!;
		sel.removeAllRanges();
		sel.addRange(range);
		await tick();
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

	function onChatKeyDown(e: KeyboardEvent) {
		if (enterToSubmit && e.key === "Enter" && !e.shiftKey && !e.ctrlKey) {
			submitDispatch("submit");
			e.preventDefault();
		}
		expectQuickPaste = e.key.toLowerCase() === "v" && e.shiftKey && e.ctrlKey;
	}

	function handlePaste(e: ClipboardEvent) {
		e.stopPropagation();
		e.preventDefault();
		const clipboardData = e.clipboardData || ((window as any).clipboardData as DataTransfer);
		//console.log(clipboardData, clipboardData.items, clipboardData.types);
		const pastedData = clipboardData.getData("Text");
		const range = window.getSelection()!.getRangeAt(0);
		const textNode = document.createTextNode(pastedData);
		range.deleteContents();
		range.insertNode(textNode);
		range.collapse(false);
		textChanged();
		if (expectQuickPaste) {
			submitDispatch("submit");
		}
	}

	onMount(() => {
		setValue = undefined;
		applyValue(value);
	});
</script>

<div class="bInput">
	<div class="input placeholder" class:invisible={value.length > 0}>
		<slot name="placeholder" />
	</div>
	<div
		bind:this={self}
		on:keydown={onChatKeyDown}
		on:input={textChanged}
		on:paste={handlePaste}
		class="input textBox"
		name="message"
		contenteditable="true" />
</div>

<style lang="scss">
	.bInput {
		width: 100%;
		position: relative;
	}

	.input.placeholder {
		position: absolute;
		background: unset;
		color: mix($text, $background, 60%);
	}

	.textBox {
		height: auto;
		display: block;
		white-space: pre-wrap;
		background-color: transparent;
		color: $text;
		word-break: break-all;
	}
</style>
