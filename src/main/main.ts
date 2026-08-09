// Control Panel entry point (PRD §6.3) — the full bundle.
//
// No router and no store are installed yet. Pinia belongs to the Control Panel
// alone (PRD §6.12) and is added by the first item that actually has state to
// keep; installing it now would be a dependency guarding nothing.

import { createApp } from 'vue'

// Before the component, so the global box model is emitted ahead of any rule
// that might one day want to override it.
import '../shared/styles/base.css'
import App from './App.vue'

createApp(App).mount('#app')
