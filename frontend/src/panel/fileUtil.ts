import { NARROW_NO_BREAK_SPACE } from "../util";

export function formatBytes(size: number): string {
	if (size < 1000) return `${size}${NARROW_NO_BREAK_SPACE}B`;
	size /= 1000;
	if (size < 1000) return `${size.toFixed()}${NARROW_NO_BREAK_SPACE}KB`;
	size /= 1000;
	if (size < 1000) return `${size.toFixed()}${NARROW_NO_BREAK_SPACE}MB`;
	size /= 1000;
	if (size < 1000) return `${size.toFixed()}${NARROW_NO_BREAK_SPACE}GB`;
	size /= 1000;
	if (size < 1000) return `${size.toFixed()}${NARROW_NO_BREAK_SPACE}TB`;
	size /= 1000;
	return `${size.toFixed()}${NARROW_NO_BREAK_SPACE}EB`;
}

export function extensionToIcon(file: string): string {
	const DEFAULT = "file-outline";
	const dotIndex = file.lastIndexOf(".");
	if (dotIndex === -1) return DEFAULT;
	const ext = file.substring(dotIndex + 1).toLowerCase();
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
		case "toml":
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

export function pathJoin(...parts: string[]): string {
	let path = "";
	for (const segment of parts) {
		const pathEndsSlash = path.endsWith("/");
		const segmentStartsSlash = segment.startsWith("/");
		if (pathEndsSlash !== segmentStartsSlash) {
			path += segment;
		} else if (pathEndsSlash && segmentStartsSlash) {
			path += segment.substring(1);
		} else {
			path += "/" + segment;
		}
	}
	return path === "" ? "/" : path;
}

export function pathSplit(...parts: string[]): string[] {
	const path = [];
	for (const segment of parts) {
		if (segment === "/") continue;
		const subSegment = segment.substring(
			segment.startsWith("/") ? 1 : 0,
			segment.endsWith("/") ? segment.length - 1 : segment.length
		);
		if (subSegment.length === 0) continue;
		if (!subSegment.includes("/")) path.push(subSegment);
		else path.push(...subSegment.split(/\//g));
	}
	return path;
}
