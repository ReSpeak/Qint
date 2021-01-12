/**
 * Describes whether the list wants elements before or after the given Element
*/
export enum ListFetchDir {
	Before,
	After,
	New,
}

export interface FetchResult<T> {
	items: T[];
	/** true if there are no more elements before the returned items */
	canLoadBeforeStart: boolean;
	/** true if there are no more elements after the returned items */
	canLoadAfterEnd: boolean;
}

export class CustomIntersectionObserver extends IntersectionObserver {
	private _observedNodes: Set<Element>;
	constructor(callback: IntersectionObserverCallback, options?: IntersectionObserverInit) {
		super(callback, options);
		this._observedNodes = new Set();
	}

	public observeWithDiff(nodes: ArrayLike<Element>) {
		let oldNodes = new Set(this._observedNodes);
		for (let i = 0; i < nodes.length; i++) {
			const element = nodes[i];
			if (this._observedNodes.has(element)) {
				oldNodes.delete(element);
			} else {
				this._observedNodes.add(element);
				super.observe(element);
			}
		}
		for (const oldNode of oldNodes) {
			super.unobserve(oldNode);
		}
	}
	public observeReplace(nodes: ArrayLike<Element>) {
		this.clear();
		this.observeNodes(nodes);
	}
	public observe(node: Element) {
		this._observedNodes.add(node);
		super.observe(node);
	}
	public observeNodes(nodes: ArrayLike<Element>) {
		for (let i = 0; i < nodes.length; i++) {
			this.observe(nodes[i]);
		}
	}
	public unobserve(node: Element) {
		this._observedNodes.delete(node);
		super.unobserve(node);
	}
	public disconnect() {
		this._observedNodes.clear();
		super.disconnect();
	}
	public clear() {
		for (let node of this._observedNodes) {
			super.unobserve(node);
		}
		this._observedNodes.clear();
	}
	public refresh() {
		for (let node of this._observedNodes) {
			super.unobserve(node);
			super.observe(node);
		}
	}
}
