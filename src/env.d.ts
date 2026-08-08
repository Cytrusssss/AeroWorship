/// <reference types="vite/client" />

// TypeScript has no built-in knowledge of `.vue` modules; this is the standard
// shim so `import App from './App.vue'` resolves. Precise per-component prop
// types still come from `vue-tsc`, which is wired up in SETUP-03.
declare module '*.vue' {
  import type { DefineComponent } from 'vue'

  const component: DefineComponent<Record<string, never>, Record<string, never>, unknown>
  export default component
}
