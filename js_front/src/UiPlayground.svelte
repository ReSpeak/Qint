<script>
	import LazyList from "./ui/LazyList.svelte";

	function* dummies() {
		for (let i = 0; ; i++) {
			yield { id: i, text: "n" + i };
		}
	}

	let elems = dummies().linq().skip(200).take(10).toArray();
</script>

<span>Before List</span>
<br />
<LazyList {elems} let:data>
	<slot>
		<span>Elem: {data.text}</span>
		<br />
	</slot>
</LazyList>
<br />
<span>After List</span>
