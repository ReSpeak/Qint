import chroma from "chroma-js";
import moment, { Duration } from "moment";
import { Moment } from "moment";
import { OffsetDateTime, RustDuration } from "./ts";
import { Readable } from "svelte/store";
import { Version } from "./book_events";

export const debug: boolean = true;
export const render_updates: boolean = false;

export const SERVER_ICON = "server";
export const CHANNEL_ICON = "chat-outline";
export const CLIENT_ICON = "account-outline";
export const BOOKMARK_ON = "star";
export const BOOKMARK_OFF = "star-outline";
export const EDIT_ICON = "pencil-outline";

export const IS_SNOWPACK = (import.meta as any).hot;
export const BASE_ADDRESS = IS_SNOWPACK ? "http://localhost:4422" : "";
export const BUILD_ENV = "__buildEnv__";
export const BUILD_DAT = "__buildDat__";
export const IS_TAURI = "__TAURI_INVOKE_HANDLER__" in window;
export const LONG_DATETIME = "dddd, MMMM Do YYYY, HH:mm:ss UTCZ";

export const NARROW_NO_BREAK_SPACE = String.fromCharCode(0x202f);
export const youtubeUrlRegex = /^((?:https?:)?\/\/)?((?:www|m)\.)?((?:youtube\.com|youtu.be))(\/(?:[\w\-]+\?v=|embed\/|v\/)?)([\w\-]+)(\S+)?$/;

export type RequiredNN<T> = { [P in keyof T]: NonNullable<T[P]> };
export type Writeable<T> = { -readonly [P in keyof T]: Writeable<T[P]> };

export async function sleep(timeout: number): Promise<void> {
	return new Promise(resolve => setTimeout(resolve, timeout));
}
export function flash(element: HTMLElement) {
	if (!element) return;
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

export function autoError(element: HTMLImageElement) {
	if (!element) return;
	function errFn(this: HTMLImageElement) {
		this.src = "/128x128.png"
		this.removeEventListener("onerror", errFn);
	}
	element.addEventListener("onerror", errFn);
}

// See https://jsperf.com/node-uuid-performance/64 about how to generate a uuid fast
export function createUuidV4(): string {
	const d2h: string[] = [], vals = new Array(16);
	for (let i = 0; i < 256; ++i) d2h.push((0x100 + i).toString(16).substr(1));

	for (let i = 0; i < 16; ++i) vals[i] = Math.random() * 256 | 0;
	vals[6] = vals[6] & 0x0f | 0x40;
	vals[8] = vals[8] & 0x3f | 0x80;
	return d2h[vals[0]] + d2h[vals[1]] + d2h[vals[2]] + d2h[vals[3]] +
		'-' + d2h[vals[4]] + d2h[vals[5]] +
		'-' + d2h[vals[6]] + d2h[vals[7]] +
		'-' + d2h[vals[8]] + d2h[vals[9]] +
		'-' + d2h[vals[10]] + d2h[vals[11]] + d2h[vals[12]] + d2h[vals[13]] + d2h[vals[14]] + d2h[vals[15]];
}

export function assert(condition: any, message: string, ...data: any[]): asserts condition {
	if (debug === false) return;
	console.assert(condition, message, ...data);
	if (!condition) debugger;
}

export function getDataColor(data: number[] | string, lightBackground: boolean = false) {
	if (data.length < 3) {
		return lightBackground ? "black" : "white";
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
	return color.css();
}

export function arraysEqual<T>(a: ArrayLike<T>, b: ArrayLike<T>): boolean {
	if (a === b) return true;
	if (a == null || b == null || a.length !== b.length)
		return false;

	for (let i = 0; i < a.length; ++i) {
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

export class Cached<TSrc, TDst> {
	private oldSource: TSrc = (Number.NaN as any);
	private value: TDst | undefined;

	constructor(
		private source: () => TSrc,
		private transform: (src: TSrc) => TDst
	) {
		this.oldSource = source();
	}

	public get(): TDst {
		const curSource = this.source();
		if (curSource !== this.oldSource) {
			this.oldSource = curSource;
			this.value = this.transform(curSource);
		}
		return this.value!;
	}
}

export function findParent(elem: HTMLElement, selector: string): HTMLElement | undefined {
	while (elem) {
		if (elem.matches(selector)) return elem;
		elem = elem.parentElement!;
	}
	return undefined;
}

export function focus(node: Element, args: any): {} {
	(node as HTMLElement).focus();
	return {};
}

export function getDefaultVersion(): Version {
	let platform = ((window.navigator as any).oscpu ?? window.navigator.userAgent).toLowerCase();
	if (platform.includes("windows")) {
		return Version.Windows_3_X_X__1;
	} else if (platform.includes("linux")) {
		return Version.Linux_3_X_X;
	} else if (platform.includes("android")) {
		return Version.Android_3_X_X;
	} else if (platform.includes("ios")) {
		return Version.iOS_3_X_X;
	} else if (platform.includes("mac")) {
		return Version.OS_X_3_X_X;
	} else {
		return Version.Windows_3_X_X__2;
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

export function urlBase64Decode(s: string): number[] {
	return base64Decode(s.replace('-', '+').replace('_', '/'));
}

export function urlBase64Encode(data: number[]): string {
	return base64Encode(data).replace('+', '-').replace('/', '_').replace(/=+$/, '');
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

export function datetimeDeserialize(rustDate: OffsetDateTime): Moment {
	return moment.unix(rustDate[0]).utcOffset(rustDate[1] / 60);
}

export function datetimeSerialize(date: Moment): OffsetDateTime {
	return [date.unix(), date.utcOffset() * 60];
}

export function durationDeserialize(rustDuration: RustDuration): Duration {
	return moment.duration(rustDuration[0] * 1000 + rustDuration[1] / 1000000);
}

export function durationSerialize(time: Duration): RustDuration {
	return [Math.floor(time.asSeconds()), Math.floor((time.asMilliseconds() % 1000) * 1000000)];
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

export function oneshot<T>(
	store: Readable<T>,
	when: (t: T) => boolean,
	action: (t: T) => void) {
	let unsub = store.subscribe(x => {
		if (when(x)) {
			unsub();
			action(x);
		}
	});
}

type FuncTyp<T extends unknown[]> = (...args: T) => void;
interface DebounceOpt {
	/**
	 * When true, resets timer on each new call. Does not fire until the timer ran out.<br>
	 * **Default**: false
	 */
	resetOnCall?: boolean,

	/**
	 * When true, calls the function once when starting the timer.<br>
	 * **Default**: false
	 */
	callInitial?: boolean,
}

export function debounced<T extends unknown[] = []>(fn: FuncTyp<T>, timeout: number, options?: DebounceOpt) {
	let timer: number | undefined;
	let lastArgs: T;
	let resetOnCall = options?.resetOnCall ?? false;
	let callInitial = options?.callInitial ?? false;

	function cancel() {
		if (timer !== undefined) {
			clearTimeout(timer);
			timer = undefined;
		}
	}

	function call(...args: T) {
		lastArgs = args;
		if (resetOnCall) {
			cancel();
		}

		if (timer === undefined) {
			timer = setTimeout(() => {
				timer = undefined;
				fn(...lastArgs);
			}, timeout);
			if (callInitial)
				fn(...args);
		}
	}

	call.cancel = cancel;
	call.call = call;
	return call;
}

export function fnBroadcast<T extends unknown[] = []>() {
	let callList: (FuncTyp<T>)[] = [];

	function call(...args: T): void {
		for (const func of callList) {
			func(...args);
		}
	}

	function clear(): void {
		callList = [];
	}

	function subscribe(func: FuncTyp<T>): () => void {
		callList.push(func);
		return () => callList.remove_item(func);
	}

	call.call = call;
	call.clear = clear;
	call.subscribe = subscribe;
	return call;
}

export function enumValues(e: object): (string | number)[] {
	return Object.keys(e) as any;
}
