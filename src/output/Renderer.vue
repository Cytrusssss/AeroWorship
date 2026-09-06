<script setup lang="ts">
const bundleName = 'AeroWorship Projector Output'
</script>

<template>
  <div class="output-surface">
    <p class="placeholder">{{ bundleName }}</p>
  </div>
</template>

<style>
html,
body {
  margin: 0;
  background: #000;

  overflow: hidden;
  user-select: none;
}
</style>

<style scoped>
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
