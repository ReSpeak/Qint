<script lang="ts">
	import { Browser, BROWSER, escapeHtml } from "../../util";
	import { createEventDispatcher, onMount, tick } from "svelte";
	import type { StructuredData } from "./uiChatInput";
	import debug from "debug";
	const log = debug("BINPUT");

	export let value: string;
	export let enterToSubmit = true;
	export let hasHistory = false;

	const dispatch = createEventDispatcher<{
		submit: undefined;
		structureChanged: undefined;
		historyMove: number;
	}>();
	let setValue: string | undefined;
	let self: HTMLElement;
	let expectQuickPaste = false;
	let hasContent = false;
	// Index into history (triggered by arrow up/down), positive
	let historyIndex: number | undefined;

	$: if (value !== setValue) {
		applyValue(value);
	}

	export async function focus(): Promise<void> {
		const range = document.createRange();
		range.selectNodeContents(self);
		const sel = window.getSelection()!;
		sel.removeAllRanges();
		sel.addRange(range);
		await tick();
		self.focus();
	}

	export function clear(): void {
		self.innerText = "";
		historyIndex = undefined;
		textChanged();
	}

	export function moveCursorToEnd(): void {
		// https://stackoverflow.com/questions/1125292/how-to-move-cursor-to-end-of-contenteditable-entity
		const range = document.createRange();
		range.selectNodeContents(self);
		range.collapse(false);
		const selection = window.getSelection();
		selection?.removeAllRanges();
		selection?.addRange(range);
	}

	function textChanged() {
		if (self === undefined) return;
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
		log("setVal:%s val:%s content:%s", setValue, value, hasContent);
		dispatch("structureChanged");
	}

	export function getStructuredView(): StructuredData {
		const parts = [];
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
						case "IMG": {
							const src =
								(node as HTMLImageElement).dataset.qintimg ??
								(node as HTMLImageElement).src;
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
							} else if (src) {
								parts.push({ src });
							}
							break;
						}
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
			textChanged();
		}
	}

	function onChatKeyDown(e: KeyboardEvent) {
		if (enterToSubmit && e.key === "Enter" && !e.shiftKey && !e.ctrlKey) {
			dispatch("submit");
			e.preventDefault();
			return;
		}
		if (hasHistory) {
			if (e.key === "ArrowDown" && historyIndex !== undefined && historyIndex !== 0) {
				historyIndex -= 1;
				if (historyIndex === 0) value = "";
				else dispatch("historyMove", historyIndex);
				e.preventDefault();
				return;
			}
			if (e.key === "ArrowUp" && (historyIndex !== undefined || value.length === 0)) {
				if (historyIndex === undefined) historyIndex = 1;
				else historyIndex += 1;
				dispatch("historyMove", historyIndex);
				e.preventDefault();
				return;
			}
		}
		historyIndex = undefined;
		expectQuickPaste = e.key?.toLowerCase() === "v" && e.shiftKey && e.ctrlKey;
		log("qick:%s shift:%s ctrl:%s", expectQuickPaste, e.shiftKey, e.ctrlKey);
	}

	function cleanNode(node: Node) {
		for (const child of node.childNodes) {
			if (child.nodeType === Node.ELEMENT_NODE) {
				cleanNode(child);
				if ((child as Element).tagName === "DIV") {
					for (const unwrap of (child as Element).childNodes) {
						node.insertBefore(document.createElement("BR"), child);
						node.insertBefore(unwrap, child);
					}
					node.removeChild(child);
				}
			}
		}
	}

	function handlePaste(e: ClipboardEvent) {
		const clipboardData = e.clipboardData || ((window as any).clipboardData as DataTransfer);

		let anyImageType: string | undefined;
		for (const item of clipboardData.items) {
			if (item.type.startsWith("image/")) {
				anyImageType = item.type;
				break;
			}
		}

		if (clipboardData.types.includes("text/plain")) {
			dispatchPaste("text/plain", clipboardData.getData("text/plain"));
		} else if (anyImageType !== undefined) {
			const urlSourceDirect = URL.createObjectURL(clipboardData.files[0]);
			handleImagePasteAsync({ type: anyImageType, url: urlSourceDirect });
		} else {
			handleImagePasteAsync(undefined);
		}
	}

	// Modern browsers don't also expose text/html when copying an image, so we try to read from the clipboard API
	async function handleImagePasteAsync(directImage: { type: string; url: string } | undefined) {
		let clipboardData: ClipboardItems | undefined = undefined;

		try {
			clipboardData = await navigator.clipboard.read();
		} catch (e) {
			log("Clipboard API not available");
		}

		if (clipboardData !== undefined && clipboardData.length > 0) {
			let clipItem = clipboardData[0];
			let anyImageType = clipItem.types.find((x) => x.startsWith("image/"));

			if (clipItem.types.includes("text/html")) {
				const blob = await clipItem.getType("text/html");
				const html = await blob.text();
				if (dispatchPaste("text/html", html)) {
					if (directImage) {
						URL.revokeObjectURL(directImage.url);
					}
					return;
				}
			}
			if (anyImageType !== undefined && directImage === undefined) {
				const blob = await clipItem.getType(anyImageType);
				directImage = { type: anyImageType, url: URL.createObjectURL(blob) };
			}
		}

		if (directImage) {
			dispatchPaste(directImage.type, directImage.url);
		}
	}

	function dispatchPaste(type: string, data: string) {
		const range = window.getSelection()!.getRangeAt(0);

		// processing clipboard data and inserting it
		if (type === "text/plain") {
			const text_plain = data;
			log("pasting as text: %s", text_plain);
			document.execCommand("insertText", false, text_plain);
			if (BROWSER !== Browser.Firefox) {
				cleanNode(self);
			}
		} else if (type === "text/html") {
			// TODO check if src: data
			const text_html = data;
			const domparser = new DOMParser();
			const dom = domparser.parseFromString(text_html, "text/html");
			const domImg = dom.querySelector("img");
			log("pasting as html %o", domImg);
			if (
				domImg?.src &&
				(domImg.src.startsWith("http://") || domImg.src.startsWith("https://"))
			) {
				const qintImg = domImg.dataset.qintimg
					? ` data-qintimg="${escapeHtml(domImg.dataset.qintimg)}"`
					: "";
				const imgHtml = `<img src="${escapeHtml(domImg.src)}"${qintImg}/>`;
				document.execCommand("insertHtml", false, imgHtml);
			} else {
				return false;
			}
		} else if (type.startsWith("image/")) {
			log("pasting as image");
			const fileUrl = data;

			const loaderImg = document.createElement("img");
			const displayImg = document.createElement("img");
			loaderImg.onload = () => {
				const canvas = document.createElement("canvas");
				canvas.width = loaderImg.naturalWidth;
				canvas.height = loaderImg.naturalHeight;
				const ctx = canvas.getContext("2d")!;
				if (type !== "image/jpeg") {
					ctx.fillStyle = "white";
					ctx.fillRect(0, 0, canvas.width, canvas.height);
				}
				ctx.drawImage(loaderImg, 0, 0);
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
			loaderImg.onerror = () => URL.revokeObjectURL(fileUrl);
			loaderImg.onabort = () => URL.revokeObjectURL(fileUrl);
			loaderImg.src = fileUrl;
			range.insertNode(displayImg);
		} else {
			log("Unknown paste type %s", type);
			return false;
		}
		// deselect the inserted data and put the cursor after it
		range.collapse(false);

		textChanged();

		if (expectQuickPaste) {
			dispatch("submit");
		}

		return true;
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
		on:paste|preventDefault|stopPropagation={handlePaste}
		class="input textBox"
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
		pointer-events: none;
		user-select: none;
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
