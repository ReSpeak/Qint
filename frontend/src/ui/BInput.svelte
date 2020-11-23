<script lang="typescript">
	import { escapeHtml } from "../util";
	import { createEventDispatcher, onMount, tick } from "svelte";
	import type { StructuredData } from "./BInputDecl";
	import debug from "debug";
	const log = debug("BINPUT");

	export let value: string;
	export let enterToSubmit = true;

	const dispatch = createEventDispatcher<{ submit: undefined; structureChanged: undefined }>();
	let setValue: string | undefined;
	let self: HTMLElement;
	let expectQuickPaste = false;
	let hasContent = false;

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

	export function clear() {
		self.innerText = "";
		textChanged();
	}

	function textChanged() {
		let tmp = self.innerText;
		if (tmp.endsWith("\n")) tmp = tmp.substring(0, tmp.length - 1);
		setValue = tmp;
		value = tmp;
		hasContent =
			self.childNodes.length > 0 &&
			!(
				self.childNodes.length === 1 &&
				(self.childNodes[0] as HTMLElement)?.tagName === "BR"
			);
		dispatch("structureChanged");
	}

	export function getStructuredView(): StructuredData {
		let parts = [];
		for (const node of self.childNodes) {
			switch (node.nodeType) {
				case Node.TEXT_NODE:
					parts.push((node as Text).data);
					break;
				case Node.ELEMENT_NODE:
					switch ((node as Element).tagName) {
						case "BR":
							parts.push("\n");
							break;
						case "IMG":
							let src = (node as HTMLImageElement).src;
							if (src.startsWith("data:")) {
								const blob = new Blob(
									[
										Uint8Array.from(atob(src.split(",")[1]), (c) =>
											c.charCodeAt(0)
										),
									],
									{ type: "image/jpeg" }
								);
								parts.push({ blob });
							} else if (src.startsWith("blob:")) {
								log("How did this blob end up here");
							} else {
								parts.push({ src });
							}
							break;
						default:
							log("Unknown node", node);
							break;
					}
					break;
			}
		}
		return parts;
	}

	function applyValue(val: string) {
		if (val !== setValue) {
			setValue = val;
			if (self) self.textContent = val;
		}
	}

	function onChatKeyDown(e: KeyboardEvent) {
		if (enterToSubmit && e.key === "Enter" && !e.shiftKey && !e.ctrlKey) {
			dispatch("submit");
			e.preventDefault();
		}
		expectQuickPaste = e.key.toLowerCase() === "v" && e.shiftKey && e.ctrlKey;
	}

	function handlePaste(e: ClipboardEvent) {
		e.stopPropagation();
		e.preventDefault();
		const clipboardData = e.clipboardData || ((window as any).clipboardData as DataTransfer);

		const types = new Set<string>();
		for (const type of clipboardData.items) {
			types.add(type.type);
		}

		if (types.size === 0) return;

		const range = window.getSelection()!.getRangeAt(0);

		// processing clipboard data and inserting it
		if (types.has("text/plain")) {
			const text_plain = clipboardData.getData("text/plain");
			log("pasting as text: %s", text_plain);
			document.execCommand("insertText", false, text_plain);
		} else if (types.has("image/png")) {
			let hasHtmlNode = false;
			if (types.has("text/html")) {
				// TODO check if src: data
				const text_html = clipboardData.getData("text/html");
				const domparser = new DOMParser();
				const dom = domparser.parseFromString(text_html, "text/html");
				const domImg = dom.querySelector("img");
				log("pasting as html %o", domImg);
				if (domImg !== null && domImg.src && !domImg.src.startsWith("file://")) {
					const imgHtml = `<img src="${escapeHtml(domImg.src)}"/>`;
					document.execCommand("insertHtml", false, imgHtml);
					hasHtmlNode = true;
				}
			}
			if (!hasHtmlNode) {
				log("pasting as image");
				const file = clipboardData.files[0];
				// TODO free object somewhen
				const fileUrl = URL.createObjectURL(file);

				const loaderImg = document.createElement("img");
				const displayImg = document.createElement("img");
				loaderImg.onload = () => {
					const canvas = document.createElement("canvas");
					canvas.width = loaderImg.naturalWidth;
					canvas.height = loaderImg.naturalHeight;
					// TODO check when contoext might be null?
					canvas.getContext("2d")!.drawImage(loaderImg, 0, 0);
					const imgData = canvas.toDataURL("image/jpeg", 0.9);
					displayImg.src = imgData;

					// const blob = new Blob(
					// 	[Uint8Array.from(atob(imgData.split(",")[1]), (c) => c.charCodeAt(0))],
					// 	{ type: "image/jpeg" }
					// );
					// log("Blob", blob);
					// TODO move to a 'final' block.
					URL.revokeObjectURL(fileUrl);
				};
				loaderImg.src = fileUrl;
				range.insertNode(displayImg);
			}
		}
		// deselect the inserted data and put the cursor after it
		range.collapse(false);

		textChanged();

		if (expectQuickPaste) {
			dispatch("submit");
		}
	}

	onMount(() => {
		setValue = undefined;
		applyValue(value);
	});
</script>

<div class="bInput">
	<div class="input placeholder" class:invisible={hasContent}>
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
		height: 100%;
	}

	.textBox {
		height: 100%;
		display: block;
		white-space: pre-wrap;
		background-color: transparent;
		color: $text;
		word-break: break-all;
		overflow-y: auto;
	}
</style>
