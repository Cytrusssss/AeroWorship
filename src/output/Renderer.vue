<script setup lang="ts">
// Placeholder for the projector surface. The real component is FR-4xx: two
// stacked layers (visible slide + preloaded next slide) driven by `slide:show`
// and `slide:preload` events, delegating the actual drawing to the single
// shared renderer in `src/shared/renderer/`.
//
// The literal below is unique to this bundle and is used as the counterpart
// marker when checking bundle separation in `dist/`.
const bundleName = 'AeroWorship Projector Output'
</script>

<template>
  <div class="output-surface">
    <p class="placeholder">{{ bundleName }}</p>
  </div>
</template>

<style>
/* Unscoped on purpose: the default 8 px body margin would put scrollbars on a
   full-screen projector surface. */
html,
body {
  margin: 0;
  background: #000;
}
</style>

<style scoped>
/* Black is the resting state of a projector, not a theme choice. */
.output-surface {
  background: #000;
  color: #fff;
  font-family: system-ui, sans-serif;
  height: 100vh;
  width: 100vw;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  cursor: none;
}

.placeholder {
  opacity: 0.35;
}
</style>
