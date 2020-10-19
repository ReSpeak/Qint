import { backend } from "../backend/backend";

const cache: Record<string, AnalyzeResult | Promise<AnalyzeResult>> = {};

const analyzeInBackend = true;

export async function analyzeLink(link: string): Promise<AnalyzeResult> {
	let result = cache[link];
	if (result === undefined) {
		let task = analyzeInBackend
			? analyzeLinkInBackend(link)
			: analyzeLinkInBrowser(link);
		cache[link] = task;
		try {
			result = await task;

			if (result.kind === "site") {
				const resultUrl = result.imageSrc;
				if (!resultUrl.startsWith("http://") && !resultUrl.startsWith("https://")) {
					let origin = new URL(link).origin;
					if (resultUrl.startsWith("/"))
						result.imageSrc = `${origin}${resultUrl}`;
					else
						result.imageSrc = `${origin}/${resultUrl}`;
				}
			}
		} catch (ex) {
			console.log("Why do you hate me?", link, ex);
			result = Unknown;
		}
		cache[link] = result;
		return result;
	} else if ("then" in result) {
		return await result;
	} else {
		return result;
	}
}

function rustResultToAnalyzeResult(data: any): AnalyzeResult {
	if (data === "Unknown") {
		return Unknown;
	} else if (data.Site) {
		return {
			kind: "site",
			title: data.Site.title,
			imageSrc: data.Site.image_src,
			description: data.Site.description
		};
	} else if (data.Image) {
		return {
			kind: "image",
			imageSrc: data.Image,
		};
	} else if (data.Video) {
		return {
			kind: "video",
			videoSrc: data.Video,
		};
	}
	throw new Error("Unknown backend result");
}

async function analyzeLinkInBackend(link: string): Promise<AnalyzeResult> {
	let result = await backend.fetch(`/peek_link/${encodeURIComponent(link)}`);
	let data = await result.json();
	return rustResultToAnalyzeResult(data);
}

async function analyzeLinkInBrowser(link: string): Promise<AnalyzeResult> {
	const response = await fetch(link, {
		mode: "cors"
	});

	const contentType = response.headers.get("content-type");
	if (!contentType)
		return Unknown;

	if (contentType.startsWith("image"))
		return {
			kind: "image",
			imageSrc: link,
		};
	else if (contentType.startsWith("text/html")) {
		const parser = new DOMParser();
		const pageDom = parser.parseFromString(await response.text(), "text/html");
		const metaTitle = pageDom.head.querySelector("meta[property='og:title']");
		if (!metaTitle) {
			// TODO we could returm more info
			return Unknown;
		}
		//const metaType = pageDom.head.querySelector("meta[property='og:type']");
		const metaImage = pageDom.head.querySelector("meta[property='og:image']");
		const metaDescription = pageDom.head.querySelector("meta[property='og:description']");
		return {
			kind: "site",
			title: metaTitle?.getAttribute("content") ?? "",
			imageSrc: metaImage?.getAttribute("content") ?? "",
			description: metaDescription?.getAttribute("content") ?? "",
		};
	}
	return Unknown;
}

const Unknown: UnknwonResult = {
	kind: undefined,
};

type AnalyzeResult = ImageResult | SiteResult | VideoResult | YoutubeResult | UnknwonResult;


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
	kind: "image",
	imageSrc: string;
}

interface VideoResult {
	kind: "video",
	videoSrc: string;
}

interface YoutubeResult {
	kind: "video_yt",
	youtube_id: string;
}
