<script lang="typescript">
	import { createEventDispatcher, tick } from "svelte";

	let useUpload = false;
	let useDownload = false;

	let uploader: HTMLInputElement;
	let downloader: HTMLIFrameElement;

	const dispatch = createEventDispatcher<{
		uploadRequest: FileList;
	}>();

	export async function askDownload(src: string, fileName?: string | null) {
		if (!useDownload) {
			useDownload = true;
			await tick();
		}
		let link = src;
		if (fileName === undefined || fileName !== null) {
			let dlName: string;
			if (fileName === undefined) {
				const lastSlash = src.lastIndexOf("/");
				if (lastSlash >= 0) {
					dlName = src.substring(lastSlash + 1);
				} else {
					dlName = "file";
				}
			} else {
				dlName = fileName;
			}
			link += `?dl=${encodeURIComponent(dlName)}`;
		}
		downloader.src = link;
	}

	export async function askUpload() {
		if (!useUpload) {
			useUpload = true;
			await tick();
		}
		uploader.click();
	}

	function uploadFilesCallback() {
		const files = uploader.files;
		if (files && files.length > 0) {
			dispatch("uploadRequest", files);
			uploader.value = null!;
		}
	}
</script>

{#if useUpload}
	<input
		title="Dummy Uploader"
		style="display: none;"
		bind:this={uploader}
		on:change={uploadFilesCallback}
		type="file"
		multiple />
{/if}
{#if useDownload}
	<iframe
		title="Dummy Downloader"
		style="display: none;"
		bind:this={downloader}
		sandbox="allow-downloads" />
{/if}
