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
		case "cue":
		case "dmg":
		case "iso":
		case "mdf":
		case "vcd":
			return "disc";
		// Keys
		case "ca-bundle":
		case "cer":
		case "crt":
		case "der":
		case "p12":
		case "p7b":
		case "p7c":
		case "p7s":
		case "pem":
		case "pfx":
			return "file-certificate-outline";
		case "gpg":
		case "kbdx":
		case "pgp":
		case "pub":
			return "file-key-outline";
		// Code (file-code-outline)
		case "c":
		case "h":
			return "language-c";
		case "cpp":
		case "cxx":
		case "hpp":
			return "language-cpp";
		case "cs":
			return "language-csharp";
		case "css":
		case "less":
		case "sass":
		case "scss":
			return "language-css3";
		case "go":
			return "language-go";
		case "hs":
			return "language-haskell";
		case "htm":
		case "html":
			return "language-html5";
		case "java":
			return "language-java";
		case "js":
			return "language-javascript";
		case "json":
			return "code-json";
		case "kt":
			return "language-kotlin";
		case "lua":
			return "language-lua";
		case "md":
			return "language-markdown-outline";
		case "php":
			return "language-php";
		case "py":
			return "language-python";
		case "r":
			return "language-r";
		case "rb":
			return "language-ruby";
		case "rs":
			return "language-rust";
		case "swift":
			return "language-swift";
		case "ts":
			return "language-typescript";
		case "xaml":
			return "language-xaml";
		case "xml":
			return "xml";
		case "ps1":
			return "powershell";
		case "sh":
			return "bash";
		case "vbs":
			return "script-text-outline";
		case "csproj":
		case "fsproj":
		case "sln":
		case "suo":
		case "vbproj":
		case "vcproj":
		case "vcxproj":
			return "microsoft-visual-studio";
		// Database
		case "db":
		case "dbf":
		case "mdb":
		case "sql":
			return "database-outline";
		// Images
		case "bmp":
		case "cr2":
		case "gif":
		case "ico":
		case "jpeg":
		case "jpg":
		case "png":
		case "psd":
		case "raw":
		case "tga":
		case "tif":
		case "tiff":
		case "webp":
			return "file-image-outline";
		case "eps":
		case "svg":
			return "vector-polyline";
		// Music
		case "aac":
		case "aif":
		case "aiff":
		case "flac":
		case "m3u":
		case "m4a":
		case "mid":
		case "mp2":
		case "mp3":
		case "mpa":
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
		case "apk":
			return "android-debug-bridge";
		case "blend":
			return "blender-software";
		case "exe":
		case "jar":
		case "msi":
			return "application-cog";
		case "pdf":
			return "file-pdf-outline";
		// Text
		case "cfg":
		case "conf":
		case "editorconfig":
		case "ini":
		case "rtf":
		case "toml":
		case "txt":
			return "file-document-outline";
		case "csv":
			return "file-delimited-outline";
		// Video
		case "3gp":
		case "avi":
		case "flv":
		case "m4v":
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
