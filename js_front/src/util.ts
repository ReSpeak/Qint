export const debug: boolean = true;

export const SERVER_ICON = "server";
export const CHANNEL_ICON = "chat-outline";
export const CLIENT_ICON = "account-outline";
export const BOOKMARK_ON = "star";
export const BOOKMARK_OFF = "star-outline";

// @ts-ignore
export const BASE_ADDRESS = ""; //"__buildEnv__" === "development" ? "http://localhost:4422" : "";

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
export function getDataColor(data: number[] | string) {
	if (data.length < 4) {
		return "";
	}
	if (typeof data === "string") {
		data = [0, 1, 2, 3, 4].map(i => (data as string).charCodeAt(i))
	}

	let varH = ((data[0] << 8) | data[1]) % 360;
	let varS = 60 + data[2] % 40; // = 80 ± 20 => [60-100]
	let varL = 30 + data[3] % 30; // = 45 ± 15 => [30- 60]
	return `color: hsl(${varH}, ${varS}%, ${varL}%);`;
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
