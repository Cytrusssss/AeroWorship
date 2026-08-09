// Projector Output entry point (PRD §6.3) — the minimal bundle.
//
// Everything reachable from this module is part of the 28 MB budget in
// PRD §5.1. It must never reach into `src/main/`, and it installs no router and
// no store. Shared code comes from `src/shared/` only.

import { createApp } from 'vue'

// The one stylesheet this bundle carries beyond its own component styles: a
// global `box-sizing: border-box` shared with the Control Panel so the two can
// never disagree about the box model. 40 bytes of minified CSS, emitted once as
// a chunk both documents link — see the file for why it exists.
import '../shared/styles/base.css'
import Renderer from './Renderer.vue'

createApp(Renderer).mount('#output')
