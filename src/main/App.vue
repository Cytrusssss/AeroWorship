<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { onMounted, ref } from 'vue'

import type { Monitor } from '../shared/bindings/Monitor'

const bundleName = 'AeroWorship Control Panel'

const monitors = ref<Monitor[]>([])
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
html,
body {
  margin: 0;
  background: #16181d;
}
</style>

<style scoped>
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

.id {
  color: #9aa1ad;
  font-family: ui-monospace, monospace;
}

.failure {
  color: #ff8a8a;
}
</style>
