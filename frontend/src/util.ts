import chroma from "chroma-js";
import moment, { Duration } from "moment";
import { Moment } from "moment";
import { OffsetDateTime, RustDuration } from "./ts";
import { Readable } from "svelte/store";
import { Version } from "./book_events";
import EMOJI_ENCODING from "./sas-emoji.json";

export const debug: boolean = true;
export const render_updates: boolean = false;

export const SERVER_ICON = "server";
export const CHANNEL_ICON = "chat-outline";
export const CLIENT_ICON = "account-outline";
export const BOOKMARK_ON = "star";
export const BOOKMARK_OFF = "star-outline";
export const EDIT_ICON = "pencil-outline";
export const CLEAR_ICON = "broom";

// the rpc field gets injected by the tauri runtime so it's a good indicator if
// we are running withing the tauri app.
export const IS_TAURI = "__TAURI__" in window;
export const LONG_DATETIME = "dddd, MMMM Do YYYY, HH:mm:ss UTCZ";
export const MIN_VOLUME_DB = -30;
export const PASSWORD_PLACEHOLDER = "**********";

export const NARROW_NO_BREAK_SPACE = String.fromCharCode(0x202f);
export const youtubeUrlRegex =
	/^((?:https?:)?\/\/)?((?:www|m)\.)?((?:youtube\.com|youtu.be))(\/(?:[\w-]+\?v=|embed\/|v\/)?)([\w-]+)(\S+)?$/;

export const VAD_MIN = 0;
export const VAD_MAX = 1;
export const LOUDNESS_MIN = -45;
export const LOUDNESS_MIN_SETTINGS = -100;
export const LOUDNESS_MAX = 0;
export const LOUDNESS_END_MAGIC = -1000;
export const LOUDNESS_HISTORY = 100;
export const LOUDNESS_UPDATE_MS = 20;
export const BROWSER = detectBrowser();

export type RequiredNN<T> = { [P in keyof T]: NonNullable<T[P]> };
export type Writeable<T> = { -readonly [P in keyof T]: Writeable<T[P]> };

export interface EmojiData {
	number: number;
	emoji: string;
	description: string;
	unicode: string;
	translated_descriptions: Record<string, string>;
}

export async function sleep(timeout: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, timeout));
}

export function flash(element: HTMLElement): void {
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

export function autoError(element: HTMLImageElement): void {
	if (!element) return;
	function errFn(this: HTMLImageElement) {
		this.src = "/128x128.png";
		this.removeEventListener("onerror", errFn);
	}
	element.addEventListener("onerror", errFn);
}

export function clickToSelectAll(element: HTMLElement): void {
	if (!element) return;
	const clickFn = function () {
		const range = document.createRange();
		range.selectNodeContents(element);
		const sel = document.getSelection();
		if (sel) {
			sel.removeAllRanges();
			sel.addRange(range);
		}
	};
	element.onfocus = clickFn;
	element.onclick = clickFn;
}

// See https://jsperf.com/node-uuid-performance/64 about how to generate a uuid fast
export function createUuidV4(): string {
	const d2h: string[] = [],
		vals = new Array(16);
	for (let i = 0; i < 256; ++i) d2h.push((0x100 + i).toString(16).substring(1));

	for (let i = 0; i < 16; ++i) vals[i] = (Math.random() * 256) | 0;
	vals[6] = (vals[6] & 0x0f) | 0x40;
	vals[8] = (vals[8] & 0x3f) | 0x80;
	return (
		d2h[vals[0]] +
		d2h[vals[1]] +
		d2h[vals[2]] +
		d2h[vals[3]] +
		"-" +
		d2h[vals[4]] +
		d2h[vals[5]] +
		"-" +
		d2h[vals[6]] +
		d2h[vals[7]] +
		"-" +
		d2h[vals[8]] +
		d2h[vals[9]] +
		"-" +
		d2h[vals[10]] +
		d2h[vals[11]] +
		d2h[vals[12]] +
		d2h[vals[13]] +
		d2h[vals[14]] +
		d2h[vals[15]]
	);
}

// eslint-disable-next-line @typescript-eslint/explicit-module-boundary-types
export function assert(condition: any, message: string, ...data: any[]): asserts condition {
	if (debug === false) return;
	console.assert(condition, message, ...data);
	if (!condition) debugger;
}

export function getDataColor(data: number[] | string, lightBackground: boolean = false): string {
	if (data.length < 3) {
		return lightBackground ? "black" : "white";
	}
	if (typeof data === "string") {
		const dataTmp = [0, 0, 0];
		for (let i = 0; i < data.length; i++)
			dataTmp[i % 3] = (dataTmp[i % 3] + data.charCodeAt(i)) % 256;
		data = dataTmp;
	}

	let color = chroma(data[0], data[1], data[2], "rgb");
	const setLum = lightBackground ? 35 : 65;
	color = color.set("lab.l", setLum);
	return color.css();
}

export function formatDuration(duration: Duration | null | undefined): string {
	if (!duration) return "";
	const asSec = Math.floor(duration.asSeconds());
	if (asSec <= 60) return `${asSec}${NARROW_NO_BREAK_SPACE}s`;
	const asMin = Math.floor(duration.asMinutes());
	const floorSec = Math.floor(duration.seconds());
	if (asMin <= 60)
		return `${asMin}${NARROW_NO_BREAK_SPACE}m ${floorSec}${NARROW_NO_BREAK_SPACE}s`;
	const asHour = Math.floor(duration.asHours());
	const floorMin = Math.floor(duration.minutes());
	if (asHour <= 24)
		return `${asHour}${NARROW_NO_BREAK_SPACE}h ${floorMin}${NARROW_NO_BREAK_SPACE}m ${floorSec}${NARROW_NO_BREAK_SPACE}s`;
	const asDay = Math.floor(duration.asDays());
	const floorHour = Math.floor(duration.hours());
	return `${asDay}${NARROW_NO_BREAK_SPACE}d ${floorHour}${NARROW_NO_BREAK_SPACE}h ${floorMin}${NARROW_NO_BREAK_SPACE}m ${floorSec}${NARROW_NO_BREAK_SPACE}s`;
}

export function arraysEqual<T>(a: ArrayLike<T>, b: ArrayLike<T>): boolean {
	if (a === b) return true;
	if (a == null || b == null || a.length !== b.length) return false;

	for (let i = 0; i < a.length; ++i) {
		if (a[i] !== b[i]) return false;
	}
	return true;
}

export function hasProperty(obj: unknown, propName: string): boolean {
	if (typeof obj !== "object" || obj === null) return false;
	return propName in obj;
}

export function escapeHtml(s: string): string {
	return s
		.replace("&", "&amp;")
		.replace("<", "&lt;")
		.replace(">", "&gt;")
		.replace('"', "&quot;")
		.replace("'", "&#x27;")
		.replace("/", "&#x2F;");
}

export function ignoreCaseRegex(search: string): RegExp {
	return RegExp(search.replace(/[-/\\^$*+?.()|[\]{}]/g, "\\$&"), "gi");
}

export class BinarySearchResult {
	public constructor(
		public found: boolean,
		// Index of found element or index where element can be inserted to maintain order
		public index: number
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
export function binarySearchBy<T>(
	list: ArrayLike<T>,
	f: (t: T) => number,
	start?: number,
	end?: number
): BinarySearchResult {
	start = start ?? 0;
	end = end ?? list.length;
	assert(start >= 0 && start <= list.length, "Start must be within list range");
	assert(end >= 0 && end <= list.length, "End must be within list range");
	assert(start <= end, "Start must be smaller than end");
	// Code is copied from Rust
	let base = start;
	let size = end - base;
	if (size === 0) return new BinarySearchResult(false, 0);

	while (size > 1) {
		const half = Math.floor(size / 2);
		const mid = base + half;
		// mid is always in [0, size), that means mid is >= 0 and < size.
		// mid >= 0: by definition
		// mid < size: mid = size / 2 + size / 4 + size / 8 ...
		const cmp = f(list[mid]);
		if (cmp <= 0) base = mid;
		size -= half;
	}
	// base is always in [0, size) because base <= mid.
	const cmp = f(list[base]);
	if (cmp === 0) return new BinarySearchResult(true, base);
	else return new BinarySearchResult(false, base + (cmp < 0 ? 1 : 0));
}

export function binarySearchByKey<T, E>(
	list: ArrayLike<T>,
	elem: E,
	f: (t: T) => E,
	start?: number,
	end?: number
): BinarySearchResult {
	return binarySearchBy(
		list,
		(t) => {
			const x = f(t);
			if (elem < x) return 1;
			if (elem > x) return -1;
			return 0;
		},
		start,
		end
	);
}

export class Lazy<T> {
	private value: T | undefined;

	constructor(private generator: () => T) { }

	public get(): T {
		if (this.generator !== undefined) {
			this.value = this.generator();
			this.generator = undefined!;
		}
		return this.value!;
	}
}

export class Cached<TSrc, TDst> {
	private oldSource: TSrc = Number.NaN as any;
	private value: TDst | undefined;

	constructor(private source: () => TSrc, private transform: (src: TSrc) => TDst) {
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

// eslint-disable-next-line @typescript-eslint/explicit-module-boundary-types
export function focus(node: Element, _args: any): Record<string, never> {
	(node as HTMLElement).focus();
	return {};
}

export function getDefaultVersion(): Version {
	const platform = ((window.navigator as any).oscpu ?? window.navigator.userAgent).toLowerCase();
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

export function iconPathToId(path: string | undefined): string {
	return path === undefined ? "0" : path.replace("icon_", "");
}

export function base64Decode(s: string): number[] {
	const res = [];
	const b = atob(s);
	for (let i = 0; i < b.length; i++) res.push(b.charCodeAt(i));
	return res;
}

export function base64Encode(data: number[]): string {
	let res = "";
	for (let i = 0; i < data.length; i++) res += String.fromCharCode(data[i]);
	return btoa(res);
}

export function urlBase64Decode(s: string): number[] {
	return base64Decode(s.replace("-", "+").replace("_", "/"));
}

export function urlBase64Encode(data: number[]): string {
	return base64Encode(data).replace("+", "-").replace("/", "_").replace(/=+$/, "");
}

export function hexDecode(s: string): number[] {
	const res = [];
	for (let i = 0; i < s.length - 1; i += 2) res.push(parseInt(s.substring(i, i + 2), 16));
	return res;
}

export function hexEncode(data: number[]): string {
	let res = "";
	for (let i = 0; i < data.length; i++) res += data[i].toString(16).padStart(2, "0");
	return res;
}

export function tsHexDecode(s: string): number[] {
	const a0 = "a".charCodeAt(0);
	const res = [];
	for (let i = 0; i < s.length - 1; i += 2)
		res.push(((s.charCodeAt(i) - a0) << 4) | (s.charCodeAt(i + 1) - a0));
	return res;
}

export function tsHexEncode(data: number[]): string {
	const a0 = "a".charCodeAt(0);
	let res = "";
	for (let i = 0; i < data.length; i++) {
		const c = data[i];
		res += String.fromCharCode(a0 + (c >> 4));
		res += String.fromCharCode(a0 + (c & 0xf));
	}
	return res;
}

// Emoji encoding from Matrix: https://matrix.org/docs/spec/client_server/latest#sas-method-emoji
// Declarations from here: https://github.com/matrix-org/matrix-doc/blob/master/data-definitions/sas-emoji.json

export function emojiEncode(data: number[]): EmojiData[] {
	const BASE64_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";
	const b64 = base64Encode(data);
	const res = [];
	for (let i = 0; i < b64.length; i++) {
		const bi = BASE64_CHARS.indexOf(b64[i]);
		if (bi !== 64) res.push(EMOJI_ENCODING[bi] as EmojiData);
	}
	return res;
}

// Java hashCode implementation for string.
// We don't need the Java version, we just need any hash and this one is short to implement.
export function javaHash(s: string): number {
	return s.split("").reduce((a, b) => {
		a = a * 31 + b.charCodeAt(0);
		return a & a; // Truncate to 32 bit
	}, 0);
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
 * Convert a factor between 0–1 to a value in dB, which is more natural.
 * See also https://www.dr-lex.be/info-stuff/volumecontrols.html
 */
export function factorToDb(factor: number): number {
	return factor === 0 ? MIN_VOLUME_DB : Math.round(20 * Math.log10(factor));
}

/**
 * Convert a value in dB to factor between 0–1, which is easier to use.
 */
export function dbToFactor(volume: number): number {
	let factor = 0;
	if (volume !== MIN_VOLUME_DB) factor = Math.pow(10, volume / 20);
	return factor;
}

/**
 * Works similar to Object.assign except that it doesn't overwrite existing
 * object structures. But instead merges them recursively
 */
// eslint-disable-next-line @typescript-eslint/explicit-module-boundary-types
export function deep_merge(obj: any, merge: any): void {
	for (const [key, value] of Object.entries(merge)) {
		if (typeof value === "object" && typeof obj[key] === "object") {
			deep_merge(obj[key], value);
		} else {
			obj[key] = value;
		}
	}
}

/**
 * Returns an object with all properties in `to` that are not already equal in from.
 * Properties in `from` that are not in `to` will be returned as `null`.
 * Returns `undefined` when both objects are identical.
 * Returns `null` if the value was deleted.
 * Returns the diff object otherwise.
 */
// eslint-disable-next-line @typescript-eslint/explicit-module-boundary-types
export function deep_diff(from: any, to: any): any | undefined {
	if (from == null) return to;
	if (to == null) return null;
	if (typeof from !== typeof to) return to;
	if (typeof to !== "object" || to === null || Array.isArray(to)) {
		if (deep_equals(from, to)) return undefined;
		return to;
	}
	let hasChanges = false;
	const res: Record<string, any> = {};
	// Check existing and new entries
	for (const [key, value] of Object.entries(to)) {
		const diff = deep_diff(from[key], value);
		if (diff !== undefined) {
			hasChanges = true;
			res[key] = diff;
		}
	}
	// Check removed entries
	for (const key of Object.keys(from)) {
		if (!(key in to)) {
			hasChanges = true;
			res[key] = null;
		}
	}
	if (!hasChanges) return undefined;
	return res;
}
(window as any).deep_diff = deep_diff;

// eslint-disable-next-line @typescript-eslint/explicit-module-boundary-types
export function deep_equals(a: any, b: any): boolean {
	if (a === b) return true;
	if (typeof a !== typeof b) return false;
	if (typeof a !== "object") return false;
	if (Array.isArray(a) && a.length !== b.length) return false;
	const a_entries = Object.entries(a);
	const b_keys = Object.keys(b);
	if (a_entries.length !== b_keys.length) return false;
	for (const [key, value] of a_entries) {
		if (!deep_equals(value, b[key])) return false;
	}
	return true;
}

// eslint-disable-next-line @typescript-eslint/no-empty-function
export function on(..._: any[]): void { }

export function oneshot<T>(
	store: Readable<T>,
	when: (t: T) => boolean,
	action: (t: T) => void
): void {
	const unsub = store.subscribe((x) => {
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
	resetOnCall?: boolean;

	/**
	 * When true, calls the function once when starting the timer.<br>
	 * **Default**: false
	 */
	callInitial?: boolean;
}

export function debounced<T extends unknown[] = []>(
	fn: FuncTyp<T>,
	timeout: number,
	options?: DebounceOpt
) {
	let timer: number | undefined;
	let lastArgs: T;
	const resetOnCall = options?.resetOnCall ?? false;
	const callInitial = options?.callInitial ?? false;

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
			timer = window.setTimeout(() => {
				timer = undefined;
				fn(...lastArgs);
			}, timeout);
			if (callInitial) fn(...args);
		}
	}

	function flush() {
		if (timer !== undefined) {
			cancel();
			fn(...lastArgs);
		}
	}

	call.cancel = cancel;
	call.call = call;
	call.flush = flush;
	return call;
}

export function fnBroadcast<T extends unknown[] = []>() {
	let callList: FuncTyp<T>[] = [];

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

	function isEmpty(): boolean {
		return callList.length === 0;
	}

	call.call = call;
	call.clear = clear;
	call.subscribe = subscribe;
	call.isEmpty = isEmpty;
	return call;
}

export function enumValues(e: Record<string, unknown>): (string | number)[] {
	return Object.keys(e) as any;
}

const SiName: readonly string[] = ["", "k", "M", "G", "T", "P", "E", "Z", "Y"];

export function formatSi(num: number, decimals: number = 0): string {
	const sign = Math.sign(num);
	let unit = Math.floor(Math.log(Math.abs(num)) / Math.log(1000));
	unit = Math.max(Math.min(unit, SiName.length - 1), 0);
	const divided = num / Math.pow(1000, unit);
	return (
		(sign * divided).toFixed(unit === 0 ? 0 : decimals) +
		(unit === 0 ? "" : NARROW_NO_BREAK_SPACE + SiName[unit])
	);
}

export enum Browser {
	Unknwon,
	Opera,
	Chrome,
	Safari,
	Firefox,
	IE,
}

function detectBrowser(): Browser {
	if ((navigator.userAgent.indexOf("Opera") || navigator.userAgent.indexOf("OPR")) != -1) {
		return Browser.Opera;
	} else if (navigator.userAgent.indexOf("Chrome") != -1) {
		return Browser.Chrome;
	} else if (navigator.userAgent.indexOf("Safari") != -1) {
		return Browser.Safari;
	} else if (navigator.userAgent.indexOf("Firefox") != -1) {
		return Browser.Firefox;
	} else if (
		navigator.userAgent.indexOf("MSIE") != -1 ||
		!!(document as any).documentMode == true
	) {
		return Browser.IE;
	} else {
		return Browser.Unknwon;
	}
}

export function nodeIsText(node: Node): node is Text {
	return node.nodeType === Node.TEXT_NODE;
}
export function nodeIsElement(node: Node): node is Element {
	return node.nodeType === Node.ELEMENT_NODE;
}

export type PromiseParts<TResolve = void, TReject = void> = {
	resolve: (res: TResolve) => void;
	reject: (rej: TReject) => void;
};
