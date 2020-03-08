import "./static/less/main.less";
import "@mdi/font/css/materialdesignicons.css";
import "bulma";
// katex
import 'katex/dist/katex.min.css'
import katex from "katex";
// highlight.js
import 'highlight.js/styles/vs2015.css';
import hljs from 'highlight.js';

import sounds from "./static/js/sounds";

// Note that a dynamic `import` statement here is required due to
// webpack/webpack#6615, but in theory `import { greet } from './pkg';`
// will work here one day as well!
const rust = import('./pkg');

rust
	.then(m => m.main())
	.catch(console.error);

window.katex = katex;
window.hljs = hljs;
window.sounds = sounds;
