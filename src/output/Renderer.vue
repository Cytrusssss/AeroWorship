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

  /* FR-105, and the two halves of it that no WebView2 setting governs.

     `overflow: hidden` is the second half of the rule above, not a repeat of
     it: the 8 px margin is why a scrollbar would appear today, but it is not
     the only way to get one, and a later item that renders a line of lyric one
     pixel taller than the display would get one anyway. There is nothing below
     the fold to scroll to on a slide.

     `user-select: none` is the whole of "no text selection". It is CSS rather
     than an API because WebView2 has no setting for it -- the context menu and
     the browser accelerator keys do, and those are switched off in Rust
     instead, in `src-tauri/src/services/webview_chrome.rs`, for reasons that
     file states. Being CSS, this one fails in the loud direction: a stylesheet
     that does not reach the window takes the black background and the sizing
     with it, so a projector that can be selected is a projector that already
     looks wrong.

     Deliberately NOT in `src/shared/styles/base.css`: that file is linked by
     the Control Panel too, and an operator who cannot select text in the song
     editor is a defect, not a hardening. */
  overflow: hidden;
  user-select: none;
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
