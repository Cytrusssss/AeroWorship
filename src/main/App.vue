<script setup lang="ts">
// Placeholder shell for the Control Panel. The real surface — Library, Session,
// Builder and Settings — arrives with the FR-2xx items and lives under `views/`.
//
// The literal below is deliberately unique to this bundle: `dist/` is grepped
// for it to prove the output entry does not pull Control Panel code in.
//
// The one thing on it that is not scaffolding is the display list: FR-101 says
// displays are enumerated at start-up by the Rust backend, and this renders the
// answer so the enumeration can be checked against Windows Display Settings
// without opening dev tools — which release builds do not have anyway (FR-105).
// It is a verification surface, not the FR-103 picker; that one is its own item
// and belongs in a settings view.
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'

import type { Monitor } from '../shared/bindings/Monitor'

const bundleName = 'AeroWorship Control Panel'

const monitors = ref<Monitor[]>([])
// Three states, not two: `null` while the command is in flight, so an empty
// list reads as "the backend reported no displays" rather than as "not asked
// yet". They are different bugs and they would look identical.
const failure = ref<string | null>(null)
const loaded = ref(false)

onMounted(async () => {
  try {
    monitors.value = await invoke<Monitor[]>('list_monitors')
  } catch (error) {
    failure.value = error instanceof Error ? error.message : String(error)
  } finally {
    loaded.value = true
  }
})
</script>

<template>
  <main class="control-panel">
    <h1>{{ bundleName }}</h1>
    <p>Frontend scaffolding only. No feature is wired up yet.</p>

    <section class="displays">
      <h2>Displays detected by the backend (FR-101)</h2>

      <p v-if="!loaded">Asking the backend…</p>
      <p v-else-if="failure" class="failure">list_monitors failed: {{ failure }}</p>
      <p v-else-if="monitors.length === 0">The backend reported no displays at all.</p>
      <template v-else>
        <p>
          {{ monitors.length }} display{{ monitors.length === 1 ? '' : 's' }} reported.
        </p>
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Resolution</th>
              <th>Position</th>
              <th>Scale</th>
              <th>Primary</th>
              <th>Id</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="monitor in monitors" :key="monitor.id">
              <td>{{ monitor.name }}</td>
              <td>{{ monitor.width }} × {{ monitor.height }}</td>
              <td>{{ monitor.x }}, {{ monitor.y }}</td>
              <td>{{ Math.round(monitor.scaleFactor * 100) }}%</td>
              <td>{{ monitor.isPrimary ? 'yes' : 'no' }}</td>
              <td class="id">{{ monitor.id }}</td>
            </tr>
          </tbody>
        </table>
      </template>
    </section>
  </main>
</template>

<style>
/* Unscoped on purpose: removes the default body margin so the dark shell
   reaches the window edges. */
html,
body {
  margin: 0;
  background: #16181d;
}
</style>

<style scoped>
/* Dark by default (NFR-27): the Control Panel is used in a darkened booth.

   `min-height: 100vh` next to `padding` only fits the viewport because of the
   global `box-sizing: border-box` in `src/shared/styles/base.css`; without it
   this pair is 100vh + 4rem tall and the window scrolls. */
.control-panel {
  background: #16181d;
  color: #e6e8ec;
  font-family: system-ui, sans-serif;
  min-height: 100vh;
  margin: 0;
  padding: 2rem;
}

.displays {
  margin-top: 2rem;
}

table {
  border-collapse: collapse;
  font-variant-numeric: tabular-nums;
}

th,
td {
  border: 1px solid #2c3038;
  padding: 0.35rem 0.75rem;
  text-align: left;
}

th {
  color: #9aa1ad;
  font-weight: 600;
}

/* The id is opaque and can be long; it is shown because this table exists to be
   compared against the OS, not because a user should ever need to read it. */
.id {
  color: #9aa1ad;
  font-family: ui-monospace, monospace;
}

.failure {
  color: #ff8a8a;
}
</style>
