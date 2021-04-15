<script lang="ts">
	import StickyList from "./ui/StickyList.svelte";
	import StickySlot from "./ui/StickySlot.svelte";
</script>

<div class="testingList">
	<StickyList>
		<StickySlot>Bavaria 1</StickySlot>
		<div>
			Bavaria ipsum dolor sit amet d’ resch Gaudi a Hoiwe im Beidl weida, Steckerleis Bladl
			wann griagd ma nacha wos z’dringa vo de! Radler von hi ghupft wia gsprunga Habedehre
			midanand so i gschmeidig. A bissal wos gehd ollaweil Vergeltsgott Kirwa, Weibaleid
			Brotzeit Leonhardifahrt amoi. Zidern Biakriagal pfiad de pfiad de is wui auf der Oim, da
			gibt’s koa Sünd. I ja, wo samma denn i moan oiwei i daad umma gscheid.
		</div>
		<StickySlot>Bavaria 2</StickySlot>
		<div>
			Bavaria ipsum dolor sit amet Jodler gar nia need middn Ohrwaschl Edlweiss glei i mog di
			fei. Anbandeln hob Lewakaas, a so a Schmarn? Hinter’m Berg san a no Leit ghupft wia
			gsprunga o’ha Biawambn blärrd pfiad de muass großherzig. Gfreit mi Reiwadatschi du dadst
			ma scho daugn de wia gar nia need mim Radl foahn weida wia da Buachbinda Wanninger! Mim
			Radl foahn oba moand trihöleridi dijidiholleri Gamsbart hob Goaßmaß Haferl, des wiad a
			Mordsgaudi scheans. Damischa wolln hogg ma uns zamm es, .
		</div>
		<StickySlot>Bavaria 3</StickySlot>
		<div>
			Bavaria ipsum dolor sit amet des wiad a Mordsgaudi o’ha bittschön Goaßmaß griasd eich
			midnand. Kimmt mei umananda blärrd middn Weißwiaschd! De da griaß God beinand,
			Oachkatzlschwoaf Schorsch ozapfa Fünferl Leonhardifahrt. I sog ja nix, i red ja bloß
			middn af, a fescha Bua! Buam Blosmusi af, Baamwach. Obandln auf der Oim, da gibt’s koa
			Sünd obacht Namidog a ganze heitzdog Blosmusi. Steckerleis boarischer i hob di liab
			damischa, sammawiedaguad.
		</div>
		<StickySlot>Bavaria 4</StickySlot>
		<div>
			Bavaria ipsum dolor sit amet des wiad a Mordsgaudi o’ha bittschön Goaßmaß griasd eich
			midnand. Kimmt mei umananda blärrd middn Weißwiaschd! De da griaß God beinand,
			Oachkatzlschwoaf Schorsch ozapfa Fünferl Leonhardifahrt. I sog ja nix, i red ja bloß
			middn af, a fescha Bua! Buam Blosmusi af, Baamwach. Obandln auf der Oim, da gibt’s koa
			Sünd obacht Namidog a ganze heitzdog Blosmusi. Steckerleis boarischer i hob di liab
			damischa, sammawiedaguad.
		</div>
	</StickyList>
</div>

<style lang="scss" global>
	@import "bulma/bulma";
	@import "bulma-slider/src/sass/index";
	@import "bulmaswatch/cyborg/_overrides";

	.testingList {
		border: 1px lightblue solid;
		height: 75vh;
		width: 20em;
	}

	.text {
		color: wheat;
		white-space: pre;
	}
</style>
