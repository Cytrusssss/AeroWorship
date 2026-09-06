import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    include: [
      'scripts/**/*.test.{js,mjs,cjs,jsx,ts,mts,cts,tsx}',
      'src/**/*.test.{js,mjs,cjs,jsx,ts,mts,cts,tsx}',
      'tests/**/*.test.{js,mjs,cjs,jsx,ts,mts,cts,tsx}',
    ],
    environment: 'node',
    globals: false,
  },
})
