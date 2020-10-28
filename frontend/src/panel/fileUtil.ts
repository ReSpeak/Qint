export function formatBytes(size: number): string {
	if (size < 1000) return `${size} B`;
	size /= 1000;
	if (size < 1000) return `${size.toFixed()} KB`;
	size /= 1000;
	if (size < 1000) return `${size.toFixed()} MB`;
	size /= 1000;
	if (size < 1000) return `${size.toFixed()} GB`;
	size /= 1000;
	if (size < 1000) return `${size.toFixed()} TB`;
	size /= 1000;
	return `${size.toFixed()} EB`;
}

export function extensionToIcon(file: string): string {
	const DEFAULT = "file-outline"
	const dotIndex = file.lastIndexOf('.');
	if (dotIndex === -1) return DEFAULT;
	let ext = file.substring(dotIndex + 1).toLowerCase();
	switch (ext) {
		// Archives
		case "7z":
		case "bz2":
		case "gz":
		case "lz":
		case "lz4":
		case "lzma":
		case "rar":
		case "tar":
		case "xz":
		case "zip":
			return "zip-box-outline";
		case "dmg":
		case "iso":
			return "disc";
		// Code
		case "c":
		case "cpp":
		case "cs":
		case "css":
		case "cxx":
		case "go":
		case "h":
		case "hpp":
		case "html":
		case "java":
		case "js":
		case "php":
		case "ps1":
		case "py":
		case "r":
		case "rs":
		case "sass":
		case "scss":
		case "sh":
		case "swift":
		case "ts":
			return "file-code-outline";
		case "jar":
			return "language-java";
		// Images
		case "bmp":
		case "cr2":
		case "gif":
		case "jpeg":
		case "jpg":
		case "png":
		case "raw":
		case "tiff":
		case "webp":
			return "file-image-outline";
		// Music
		case "aac":
		case "flac":
		case "m4a":
		case "mp3":
		case "ogg":
		case "opus":
		case "wav":
		case "wma":
			return "file-music-outline";
		// Office
		case "ods":
			return "file-table-outline";
		case "odp":
		case "pps":
		case "ppsx":
		case "ppt":
		case "pptx":
			return "file-powerpoint-outline";
		case "odt":
		case "doc":
		case "docx":
			return "file-word-outline";
		case "xls":
		case "xlsx":
			return "file-excel-outline";
		// Other
		case "exe":
			return "application-cog";
		case "pdf":
			return "file-pdf-outline";
		// Text
		case "cfg":
		case "editorconfig":
		case "ini":
		case "txt":
			return "file-document-outline";
		case "csv":
			return "file-delimited-outline";
		// Video
		case "avi":
		case "flv":
		case "mkv":
		case "mov":
		case "mp4":
		case "webm":
		case "wmv":
			return "file-video-outline";
		default:
			return DEFAULT;
	}
}
