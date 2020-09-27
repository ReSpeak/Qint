import chroma from "chroma-js";
import { OMsgConnect } from "./backend/ws";
export const debug: boolean = true;


export const SERVER_ICON = "server";
export const CHANNEL_ICON = "chat-outline";
export const CLIENT_ICON = "account-outline";
export const BOOKMARK_ON = "star";
export const BOOKMARK_OFF = "star-outline";
export const EDIT_ICON = "pencil-outline";

// @ts-ignore
export const BASE_ADDRESS = ""; //"__buildEnv__" === "development" ? "http://localhost:4422" : "";
export const BUILD_ENV = "__buildEnv__";
export const BUILD_DAT = "__buildDat__";

export async function sleep(timeout: number): Promise<void> {
	return new Promise(resolve => setTimeout(resolve, timeout));
}
export function flash(element: HTMLElement) {
	requestAnimationFrame(() => {
		element.classList.remove("update-flash-fade");
		element.classList.add("update-flash");

		setTimeout(() => {
			element.classList.add("update-flash-fade");
			element.classList.remove("update-flash");
		});

		setTimeout(() => {
			element.classList.remove("update-flash-fade");
		}, 1000);
	});
}

export function assert(condition: any, message: string, ...data: any[]): asserts condition {
	if (debug === false) return;
	console.assert(condition, message, ...data);
	if (!condition) debugger;
}

export function getDataColor(data: number[] | string, lightBackground: boolean = false) {
	if (data.length < 3) {
		return lightBackground ? "color: black;" : "color: white;";
	}
	if (typeof data === "string") {
		const dataTmp = [0, 0, 0];
		for (let i = 0; i < data.length; i++)
			dataTmp[i % 3] = (dataTmp[i % 3] + data.charCodeAt(i)) % 256;
		data = dataTmp;
	}

	let color = chroma(data[0], data[1], data[2], 'rgb');
	const setLum = lightBackground ? 35 : 65;
	color = color.set("lab.l", setLum);
	return `color: ${color.css()};`;
}

export function arraysEqual<T>(a: ArrayLike<T>, b: ArrayLike<T>): boolean {
	if (a === b) return true;
	if (a == null || b == null || a.length !== b.length)
		return false;

	for (var i = 0; i < a.length; ++i) {
		if (a[i] !== b[i])
			return false;
	}
	return true;
}

export function escapeHtml(s: string) {
	return s.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&#x27;")
		.replace('/', "&#x2F;");
}

export function ignoreCaseRegex(search: string) {
	return RegExp(search.replace(/[-\/\\^$*+?.()|[\]{}]/g, "\\$&"), "gi");
}

export class BinarySearchResult {
	public constructor(
		public found: boolean,
		// Index of found element or index where element can be inserted to maintain order
		public index: number,
	) { }
}

/** The comparator function should implement an order consistent with the sort order of the underlying slice,
 * returning an order code that indicates whether its argument is
 * less (< 0), equal (0) or greater (> 0) the desired target.
 * @param list the list to search through
 * @param f the compare function, indicating whether the passed element is bigger or smaller than the target.
 * @param [start] The starting index for the search. 0 by default.
 * @param [end] The end index for the search. End of the list by default.
 */
export function binarySearchBy<T>(list: ArrayLike<T>, f: (t: T) => number, start?: number, end?: number): BinarySearchResult {
	start = start ?? 0;
	end = end ?? list.length;
	assert(start >= 0 && start <= list.length, "Start must be within list range");
	assert(end >= 0 && end <= list.length, "End must be within list range");
	assert(start <= end, "Start must be smaller than end");
	// Code is copied from Rust
	let base = start;
	let size = end - base;
	if (size === 0)
		return new BinarySearchResult(false, 0);

	while (size > 1) {
		const half = Math.floor(size / 2);
		const mid = base + half;
		// mid is always in [0, size), that means mid is >= 0 and < size.
		// mid >= 0: by definition
		// mid < size: mid = size / 2 + size / 4 + size / 8 ...
		const cmp = f(list[mid]);
		if (cmp <= 0)
			base = mid;
		size -= half;
	}
	// base is always in [0, size) because base <= mid.
	const cmp = f(list[base]);
	if (cmp === 0)
		return new BinarySearchResult(true, base);
	else
		return new BinarySearchResult(false, base + (cmp < 0 ? 1 : 0));
}

export function binarySearchByKey<T, E>(list: ArrayLike<T>, elem: E, f: (t: T) => E, start?: number, end?: number): BinarySearchResult {
	return binarySearchBy(list, t => {
		const x = f(t);
		if (elem < x)
			return 1;
		if (elem > x)
			return -1;
		return 0;
	}, start, end);
}

export class Lazy<T> {
	private value: T | undefined;

	constructor(
		private generator: () => T
	) { }

	public get(): T {
		if (this.generator !== undefined) {
			this.value = this.generator();
			this.generator = undefined!;
		}
		return this.value!;
	}
}

export async function wrap_async<T>(f: () => T): Promise<T> {
	return new Promise((resolve, _) => {
		setTimeout(() => {
			const val = f();
			resolve(val);
		});
	});
}

export function findParent(elem: HTMLElement, selector: string): HTMLElement | undefined {
	while (elem) {
		if (elem.matches(selector)) return elem;
		elem = elem.parentElement!;
	}
	return undefined;
}

export function getDefaultVersion(): string {
	let platform = ((window.navigator as any).oscpu ?? window.navigator.userAgent).toLowerCase();
	if (platform.includes("windows")) {
		return "Windows_3_X_X__1";
	} else if (platform.includes("linux")) {
		return "Linux_3_X_X";
	} else if (platform.includes("android")) {
		return "Android_3_X_X";
	} else if (platform.includes("ios")) {
		return "iOS_3_X_X";
	} else if (platform.includes("mac")) {
		return "OS_X_3_X_X";
	} else {
		return "Windows_3_X_X__2";
	}
}

export function getConnectFromString(loc: string): OMsgConnect {
	if (loc.startsWith("{")) {
		// Parse json
		console.log(loc);
		let data = JSON.parse(loc);
		assert("address" in data, "connection data needs an address");
		if (!("name" in data))
			data.name = "TeamSpeakUser";
		if (!("version" in data))
			data.version = getDefaultVersion();
		if (!("ignore_identity_mismatch" in data))
			data.ignore_identity_mismatch = false;
		if (!("log_commands" in data))
			data.log_commands = false;
		if (!("log_packets" in data))
			data.log_packets = false;
		if (!("log_udp_packets" in data))
			data.log_udp_packets = false;
		return { Connect: data };
	} else {
		let start = loc.indexOf("@");
		let name = start === -1 ? "TeamSpeakUser" : loc.substr(0, start);
		start += 1;
		let end = loc.indexOf("/");
		let channel = end === -1 ? "" : loc.substr(end + 1);
		let address = loc.substr(start, end === -1 ? undefined : end);
		return {
			Connect: {
				bookmark: undefined,
				address,
				name,
				channel,
				version: getDefaultVersion(),
				ignore_identity_mismatch: false,
				log_commands: false,
				log_packets: false,
				log_udp_packets: false,
			}
		};
	}
}

export function getStringFromConnect(connect: OMsgConnect): string {
	const c: any = connect.Connect;
	let hasDefaults = c.bookmark === undefined;
	if (c.version === getDefaultVersion())
		c.version = undefined;
	else
		hasDefaults = false;
	if (!c.ignore_identity_mismatch)
		c.ignore_identity_mismatch = undefined;
	else
		hasDefaults = false;
	if (!c.log_commands)
		c.log_commands = undefined;
	else
		hasDefaults = false;
	if (!c.log_packets)
		c.log_packets = undefined;
	else
		hasDefaults = false;
	if (!c.log_udp_packets)
		c.log_udp_packets = undefined;
	else
		hasDefaults = false;
	if (hasDefaults) {
		let s = "";
		if (c.name !== "TeamSpeakUser")
			s = c.name + "@";
		s += c.address;
		if (c.channel !== undefined)
			s += "/" + c.channel;
		return s;
	} else {
		return JSON.stringify(c);
	}
}

export function base64Decode(s: string): number[] {
	let res = [];
	const b = atob(s);
	for (let i = 0; i < b.length; i++)
		res.push(b.charCodeAt(i));
	return res;
}

export function base64Encode(data: number[]): string {
	let res = "";
	for (let i = 0; i < data.length; i++)
		res += String.fromCharCode(data[i]);
	return btoa(res);
}

export function hexDecode(s: string): number[] {
	let res = [];
	for (let i = 0; i < s.length; i += 2)
		res.push(parseInt(s.substr(i, i + 2), 16));
	return res;
}

export function hexEncode(data: number[]): string {
	let res = "";
	for (let i = 0; i < data.length; i++)
		res += data[i].toString(16).padStart(2, "0");
	return res;
}

/**
 * Works similar to Object.assign except that it doesn't overwrite existing
 * object structures. But instead merges them recursively
*/
export function soft_merge(obj: any, merge: any) {
	for (const [key, value] of Object.entries(merge)) {
		if (typeof obj[key] === "object") {
			soft_merge(obj[key], value);
		} else {
			obj[key] = value;
		}
	}
}

export function on(..._: any[]) { }
