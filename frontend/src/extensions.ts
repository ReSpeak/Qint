// eslint-disable-next-line @typescript-eslint/no-unused-vars
interface Array<T> {
	remove_item(o: T): void;
}

Object.defineProperty(Array.prototype, "remove_item", {
	value<T>(this: T[], item: T): void {
		for (let i = this.length - 1; i >= 0; i--) {
			if (this[i] === item) {
				this.splice(i, 1);
			}
		}
	},
});

interface Math {
	clamp(val: number, min: number, max: number): number;
}

Math.clamp = function (a, b, c) {
	return Math.max(b, Math.min(c, a));
};
