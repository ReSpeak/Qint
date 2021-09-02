<script lang="ts">
	import type { PromiseParts } from "../../util";

	let uploader: HTMLInputElement;
	let downloader: HTMLIFrameElement;

	let callback: PromiseParts<FileList> | undefined;

	export async function askDownload(url: string): void {
		downloader.src = url;
	}

	export async function askUpload(multiple: boolean): Promise<FileList> {
		if (callback !== undefined) {
			callback.reject();
		}
		const promise = new Promise<FileList>((resolve, reject) => {
			callback = { resolve, reject };
		});

		uploader.value = null!;
		uploader.multiple = multiple;
		uploader.click();
		return await promise;
	}

	function uploadFilesCallback() {
		try {
			const files = uploader.files;
			if (files && files.length > 0) {
				callback?.resolve(files);
			} else {
				callback?.reject();
			}
		} finally {
			callback = undefined;
		}
	}
</script>

<input
	title="Dummy Uploader"
	style="display: none;"
	bind:this={uploader}
	on:change={uploadFilesCallback}
	type="file"
	multiple />
<iframe
	title="Dummy Downloader"
	style="display: none;"
	bind:this={downloader}
	sandbox="allow-downloads" />
