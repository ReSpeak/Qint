import { IFileRequest } from "../../backend/backend";
import { pathJoin } from "../../panel/fileUtil";

export type LinksMap = Map<
	string,
	{
		link: string;
		title: string;
	}
>;

const ts3Scheme = /^(ts3file|ts3image):\/\/([^?]*)(\?(.*))?$/i;

type Ts3Scheme =
	| {
			scheme: "ts3file";
			server: string;
			attrs: Partial<Ts3FileAtt>;
	  }
	| {
			scheme: "ts3image";
			file: string;
			attrs: Partial<Ts3ImageAtt>;
	  };
type Ts3FileAtt = {
	port: string;
	serverUID: string;
	channel: string;
	path: string;
	filename: string;
	isDir: string;
	fileDateTime: string;
};

type Ts3ImageAtt = {
	channel: string;
	path: string;
};

export function parseTsScheme(url: string): Ts3Scheme | null {
	const m = ts3Scheme.exec(url);
	if (m === null) return null;
	const schemeStr = m[1];
	const queryPart = m[4];
	const hostPart = m[2];
	if (!queryPart || !hostPart) return null;
	const params: Record<string, string> = {};
	for (const param of queryPart.split("&")) {
		const eqIndex = param.indexOf("=");
		if (eqIndex === -1) continue;
		const key = param.substring(0, eqIndex);
		const value = decodeURIComponent(param.substring(eqIndex + 1));
		params[key] = value;
	}
	if (schemeStr === "ts3file") {
		return {
			scheme: schemeStr,
			server: hostPart,
			attrs: params,
		};
	} else if (schemeStr === "ts3image") {
		return {
			scheme: schemeStr,
			file: hostPart,
			attrs: params,
		};
	}
	return null;
}

export function schemeToLink(scheme: Ts3Scheme): IFileRequest | null {
	if (scheme.attrs.path) {
		let path: string;
		let channel: string;
		let suggested_name: string | undefined;
		if (scheme.scheme === "ts3file") {
			channel = scheme.attrs.channel!;
			path = pathJoin(scheme.attrs.path, scheme.attrs.filename ?? "");
			suggested_name = scheme.attrs.filename;
		} else if (scheme.scheme === "ts3image") {
			channel = scheme.attrs.channel!;
			path = pathJoin(scheme.attrs.path, scheme.file);
			suggested_name = guessName(path);
		} else {
			throw new Error("Not supported scheme");
		}
		return { channel, path, cache: true, suggested_name };
	}
	return null;
}

export function guessName(src: string | undefined | null): string | undefined {
	if (!src) return undefined;
	const lastSlash = src.lastIndexOf("/");
	if (lastSlash >= 0) {
		return src.substring(lastSlash + 1);
	} else {
		return src;
	}
}
