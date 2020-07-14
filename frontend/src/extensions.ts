// tslint:disable-next-line: interface-name
interface Array<T> {
	remove_item(o: T): void;
	linq(): Linqerator<T, void, unknown>;
}

Object.defineProperty(Array.prototype, "remove_item", {
	value<T>(this: T[], item: T): void {
		for (let i = this.length - 1; i >= 0; i--) {
			if (this[i] === item) {
				this.splice(i, 1);
			}
		}
	}
});

// ******** LINQ ********

interface Generator<T = unknown, TReturn = any, TNext = unknown> {
	linq(): Linqerator<T, TReturn, TNext>;
}
interface Iterable<T> {
	linq(): Linqerator<T, void, unknown>;
}
interface IterableIterator<T> extends Iterator<T> {
	linq(): Linqerator<T, void, unknown>;
}

Object.defineProperty(Object.prototype, "linq", {
	value<T, TReturn, TNext>(this: Generator<T, TReturn, TNext>): Linqerator<T, TReturn, TNext> {
		return new Linqerator(this[Symbol.iterator]());
	}
});

// A Linq-like iterator
class Linqerator<T, TReturn = any, TNext = undefined> implements Generator<T, TReturn, TNext> {
	private gen: Generator<T, TReturn, TNext>;

	public linq(): Linqerator<T, TReturn, TNext> { return this; }
	public next(...args: [] | [TNext]): IteratorResult<T, TReturn> { return this.gen.next(...args); }
	public return(value: TReturn): IteratorResult<T, TReturn> { return this.gen.return(value); }
	public throw(e: any): IteratorResult<T, TReturn> { return this.gen.throw(e); }
	public [Symbol.iterator](): Generator<T, TReturn, TNext> { return this.gen[Symbol.iterator](); }

	constructor(iter: Generator<T, TReturn, TNext>) {
		this.gen = iter;
	}

	public take(count: number): Linqerator<T, TReturn | void, unknown> {
		function* take_iter(iter: Generator<T, TReturn, TNext>, icount: number): Generator<T, TReturn | void, unknown> {
			for (let i = 0; i < icount; i++) {
				const res = iter.next();
				if (res.done) return res.value;
				yield res.value;
			}
		}
		return new Linqerator<T, TReturn | void, unknown>(take_iter(this.gen, count));
	}

	public skip(count: number): Linqerator<T, TReturn, unknown> {
		function* skip_iter(iter: Generator<T, TReturn, TNext>, icount: number): Generator<T, TReturn, unknown> {
			for (let i = 0; i < icount; i++) {
				const res = iter.next();
				if (res.done) return res.value;
			}
			while (true) {
				const res = iter.next();
				if (res.done) return res.value;
				yield res.value;
			}
		}
		return new Linqerator<T, TReturn, unknown>(skip_iter(this.gen, count));
	}

	public toArray(): T[] {
		return [...(this.gen as Generator<T, TReturn, unknown>)];
	}
}
