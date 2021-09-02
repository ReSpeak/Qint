import { backend } from "../backend/backend";
import { youtubeUrlRegex } from "../util";

const cache: Record<string, AnalyzeResult | Promise<AnalyzeResult>> = {};

export async function analyzeLink(link: string): Promise<AnalyzeResult> {
	let result = cache[link];
	if (result === undefined) {
		if (youtubeUrlRegex.test(link)) {
			result = {
				kind: "video",
				videoSrc: link,
				embed: "youtube",
			};
		} else {
			const task = analyzeLinkInBackend(link);
			cache[link] = task;
			try {
				result = await task;

				if (result.kind === "site") {
					const resultUrl = result.imageSrc;
					const origin = new URL(link).origin;
					if (resultUrl.startsWith("//")) result.imageSrc = `https:${resultUrl}`;
					else if (resultUrl.startsWith("/")) result.imageSrc = `${origin}${resultUrl}`;
					else if (!resultUrl.startsWith("http://") && !resultUrl.startsWith("https://"))
						result.imageSrc = `${origin}/${resultUrl}`;
				}
			} catch (ex) {
				console.log("Why do you hate me?", link, ex);
				result = Unknown;
			}
		}
		cache[link] = result;
		return result;
	} else if ("then" in result) {
		return await result;
	} else {
		return result;
	}
}

function rustResultToAnalyzeResult(data: RustAnalyzeResult): AnalyzeResult {
	if (data === "Unknown") {
		return Unknown;
	} else if ("Site" in data) {
		return {
			kind: "site",
			title: data.Site.title,
			imageSrc: data.Site.image_src,
			description: data.Site.description ?? "",
		};
	} else if ("Image" in data) {
		return {
			kind: "image",
			imageSrc: data.Image,
		};
	} else if ("Video" in data) {
		return {
			kind: "video",
			videoSrc: data.Video,
		};
	}
	throw new Error("Unknown backend result");
}

async function analyzeLinkInBackend(link: string): Promise<AnalyzeResult> {
	const data = await backend.peek_link(link);
	return rustResultToAnalyzeResult(data);
}

const Unknown: UnknwonResult = {
	kind: undefined,
};

type AnalyzeResult = ImageResult | SiteResult | VideoResult | UnknwonResult;
export type EmbedTypes = "youtube";

export type RustAnalyzeResult =
	| "Unknown"
	| { Image: string }
	| { Video: string }
	| { Site: { title: string; image_src: string; description: string | null } };

interface UnknwonResult {
	kind: undefined;
}

interface SiteResult {
	kind: "site";
	title: string;
	imageSrc: string;
	description: string;
}

interface ImageResult {
	kind: "image";
	imageSrc: string;
}

interface VideoResult {
	kind: "video";
	videoSrc: string;
	embed?: EmbedTypes;
}
