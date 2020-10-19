const cache: Record<string, AnalyzeResult | Promise<AnalyzeResult>> = {};
console.log("PRE", cache);
export async function analyzeLink(link: string): Promise<AnalyzeResult> {
	let result = cache[link];
	if (result === undefined) {
		const task = analyzeLinkInternal(link);
		cache[link] = task;
		try {
			result = await task;
		} catch (ex) {
			console.log("Why do you hate me website?", link, ex);
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

async function analyzeLinkInternal(link: string): Promise<AnalyzeResult> {
	const response = await fetch(link, {
		mode: "cors"
	});

	const contentType = response.headers.get("content-type");
	if (!contentType)
		return Unknown;

	if (contentType.startsWith("image"))
		return {
			kind: "image",
			imageSrc: link
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

type AnalyzeResult = ImageResult | SiteResult | UnknwonResult;

interface UnknwonResult {
	kind: undefined;
}

interface ImageResult {
	kind: "image",
	imageSrc: string;
}

interface SiteResult {
	kind: "site";
	title: string;
	imageSrc: string;
	description: string;
}
