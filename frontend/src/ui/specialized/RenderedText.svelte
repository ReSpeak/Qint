<script lang="ts">
	import ImageModal from "../../chat/ImageModal.svelte";
	import FileIO from "../util/FileIO.svelte";
	import katex from "katex";
	import { hljsHighlight } from "../util/hljs";
	import { onMount } from "svelte";
	import { parseTsScheme, schemeToLink } from "./uiRenderedText";
	import type { LinksMap } from "./uiRenderedText";
	import type { IConnection } from "../../connection";
	import { extensionToIcon } from "../../panel/fileUtil";

	export let connection: IConnection;
	export let text: string;
	export let links: LinksMap = new Map();

	let showBig = false;
	let showBigSrc = "";
	let rendered: HTMLElement;
	let hasDownload = false;
	let fileIo: FileIO;
	$: renderedObj = render(text);

	function render(html: string) {
		const obj = document.createElement("div");
		obj.innerHTML = html;

		// Apply highlight.js
		for (const elem of obj.getElementsByTagName("code")) {
			hljsHighlight(elem);
		}

		// Apply KaTeX
		for (const elem of obj.getElementsByClassName("latex") as any as HTMLElement[]) {
			const code = elem.getAttribute("data-latex");
			const mode = elem.getAttribute("data-displaymode");
			try {
				if (code) {
					katex.render(code, elem, {
						displayMode: mode === "true",
						throwOnError: false,
					});
				}
			} catch {
				console.error("Failed to render latex");
				elem.textContent = code ?? "";
			}
		}

		links.clear();

		// Process links and images
		for (const a of obj.querySelectorAll("a")) {
			const href = a.href;
			if (!href || links.has(href)) continue;

			const scheme = parseTsScheme(href);
			if (scheme?.scheme === "ts3file") {
				hasDownload = true;
				a.classList.add("file_download");
				a.onclick = function (e) {
					e.preventDefault();
					const proxyFileSrc = schemeToLink(connection, scheme);
					if (proxyFileSrc !== null) {
						fileIo.askDownload(proxyFileSrc, scheme.attrs.filename);
					}
				};
				a.insertAdjacentHTML(
					"afterbegin",
					`<span class="icon" style="font-size: 1.5em;">
						<i class="mdi mdi-${extensionToIcon(scheme.attrs.filename ?? "")}"></i>
					</span>`
				);
			} else {
				links.set(href, {
					link: href,
					title: a.textContent ?? "",
				});
			}
		}

		// process ts3file links
		for (const img of obj.querySelectorAll("img")) {
			const src = img.src;
			if (!src) continue;

			img.classList.add("limitChatSize", "previewImg", "padTop");
			img.onclick = () => {
				showBigSrc = img.src;
				showBig = true;
			};
			img.dataset.qintimg = src;

			const scheme = parseTsScheme(src);
			if (scheme !== null) {
				const req = schemeToLink(connection, scheme);
				if (req !== null) {
					req.con
						.fileProvider(req)
						.then((proxyFileSrc) => {
							if (proxyFileSrc === undefined) {
								img.parentElement?.removeChild(img);
							} else {
								img.src = proxyFileSrc;
							}
						})
						.catch((err) => {
							console.warn("Failed to load", err, req);
						});
				}
			}
		}

		if (links.size > 0) {
			links = links;
		}

		if (rendered) {
			rendered.innerHTML = "";
			rendered.appendChild(obj);
		}
		return obj;
	}

	onMount(() => {
		rendered.innerHTML = "";
		rendered.appendChild(renderedObj);
	});
</script>

<div class="textRendered content" bind:this={rendered} />
{#if showBig}
	<ImageModal src={showBigSrc} bind:visible={showBig} />
{/if}
{#if hasDownload}
	<FileIO bind:this={fileIo} />
{/if}

<style lang="scss">
	.textRendered {
		white-space: pre-wrap;
		word-wrap: break-word;
		margin-bottom: 0 !important;

		:global(.para:not(:last-child)) {
			margin-bottom: 1em;
		}

		:global(code.hljs) {
			display: inline;
			padding: 0.1em;
		}

		:global(pre code.hljs) {
			display: block;
			padding: 0.5em;
			position: relative;
			overflow-x: scroll;
		}

		:global([data-codelang]::before) {
			font-size: 0.85em;
			content: attr(data-codelang);
			position: absolute;
			z-index: 2;
			bottom: 0;
			right: 3px;
			color: $orange;
			font-weight: bold;
			font-family: Sans-Serif;
			text-transform: uppercase;
			pointer-events: none;
		}
	}

	:global(.file_download) {
		&:hover {
			text-decoration: none;
		}

		display: inline-flex;
		align-items: center;

		background-color: $grey-accent;
		border: 1px solid $box-background-color;
		border-radius: 3px;

		padding: 0.5em;
		margin: 0 0.5em;
	}
</style>
