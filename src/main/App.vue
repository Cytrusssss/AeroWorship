<script setup lang="ts">
// Placeholder shell for the Control Panel. The real surface — Library, Session,
// Builder and Settings — arrives with the FR-1xx/FR-2xx items and lives under
// `views/`.
//
// The literal below is deliberately unique to this bundle: `dist/` is grepped
// for it to prove the output entry does not pull Control Panel code in.
const bundleName = 'AeroWorship Control Panel'
</script>

<template>
  <main class="control-panel">
    <h1>{{ bundleName }}</h1>
    <p>Frontend scaffolding only. No feature is wired up yet.</p>
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
</style>
