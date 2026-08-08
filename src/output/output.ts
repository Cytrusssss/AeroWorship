// Projector Output entry point (PRD §6.3) — the minimal bundle.
//
// Everything reachable from this module is part of the 28 MB budget in
// PRD §5.1. It must never reach into `src/main/`, and it installs no router and
// no store. Shared code comes from `src/shared/` only.

import { createApp } from 'vue'

import Renderer from './Renderer.vue'

createApp(Renderer).mount('#output')
