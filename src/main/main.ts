// Control Panel entry point (PRD §6.3) — the full bundle.
//
// No router and no store are installed yet. Pinia belongs to the Control Panel
// alone (PRD §6.12) and is added by the first item that actually has state to
// keep; installing it now would be a dependency guarding nothing.

import { createApp } from 'vue'

import App from './App.vue'

createApp(App).mount('#app')
