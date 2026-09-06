import js from '@eslint/js'
import prettier from 'eslint-config-prettier/flat'
import vue from 'eslint-plugin-vue'
import globals from 'globals'
import tseslint from 'typescript-eslint'

const BUNDLE_SEPARATION_RATIONALE =
  'src/output/ is a separate Vite entry whose size backs the 28 MB Projector ' +
  'Output line in the PRD §5.1 memory budget, and the bundle separation is a ' +
  'contract (PRD §6.3).'

const SEGMENT_MATCH_CAVEAT =
  'If this import does not point at src/main/ at all, the rule matched a ' +
  'specifier whose own main is a standalone path segment (ADR-0021). Renaming ' +
  'that file or directory clears it; for a dependency subpath, which cannot be ' +
  'renamed, silence it with an eslint-disable comment naming the rule that ' +
  'fired, or narrow the entry for that rule in eslint.config.js.'

const OUTPUT_ISOLATION_MESSAGE =
  'The projector bundle (src/output/) must not import from src/main/. ' +
  `${BUNDLE_SEPARATION_RATIONALE} Shared code belongs in src/shared/. ` +
  SEGMENT_MATCH_CAVEAT

const SHARED_ISOLATION_MESSAGE =
  'src/shared/ must not import from src/main/. It is consumed by the projector ' +
  'bundle, so this import is pulled into src/output/ too: the path ' +
  'src/output/ → src/shared/ → src/main/ lands exactly the code that the direct ' +
  `import is forbidden for. ${BUNDLE_SEPARATION_RATIONALE} Either move the piece ` +
  'you need down into src/shared/, or leave it in src/main/ and import it only ' +
  `from src/main/. ${SEGMENT_MATCH_CAVEAT}`

const MAIN_IMPORT_GROUPS = ['**/main', '**/main/**']

const DYNAMIC_MAIN_IMPORT_SELECTORS = [
  String.raw`ImportExpression > Literal[value=/(^|\/)main(\/|$)/]`,
  String.raw`ImportExpression > TemplateLiteral > TemplateElement[value.cooked=/(^|\/)main(\/|$)/]`,
]

const SOURCE_EXTENSIONS = 'js,mjs,cjs,jsx,ts,mts,cts,tsx,vue'

const BUNDLED_SOURCES = `**/*.{${SOURCE_EXTENSIONS}}`

const COLLOCATED_TESTS = `src/**/*.test.{${SOURCE_EXTENSIONS}}`

export default tseslint.config(
  {
    ignores: [
      'dist/',
      'node_modules/',
      'src-tauri/target/',
      'src-tauri/gen/',
      'src/shared/bindings/',
    ],
  },

  js.configs.recommended,
  tseslint.configs.recommended,
  vue.configs['flat/recommended'],

  {
    files: [`src/${BUNDLED_SOURCES}`],
    ignores: [COLLOCATED_TESTS],
    languageOptions: {
      globals: globals.browser,
    },
  },

  {
    files: ['**/*.vue'],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
      },
    },
  },

  {
    files: [
      `scripts/${BUNDLED_SOURCES}`,
      `tests/${BUNDLED_SOURCES}`,
      `*.config.{${SOURCE_EXTENSIONS}}`,
      COLLOCATED_TESTS,
    ],
    languageOptions: {
      globals: globals.node,
    },
  },

  {
    files: ['src/main/App.vue', 'src/output/Renderer.vue'],
    rules: {
      'vue/multi-word-component-names': 'off',
    },
  },

  {
    files: [`src/output/${BUNDLED_SOURCES}`],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [{ group: MAIN_IMPORT_GROUPS, message: OUTPUT_ISOLATION_MESSAGE }],
        },
      ],
      'no-restricted-syntax': [
        'error',
        ...DYNAMIC_MAIN_IMPORT_SELECTORS.map((selector) => ({
          selector,
          message: OUTPUT_ISOLATION_MESSAGE,
        })),
      ],
    },
  },

  {
    files: [`src/shared/${BUNDLED_SOURCES}`],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [{ group: MAIN_IMPORT_GROUPS, message: SHARED_ISOLATION_MESSAGE }],
        },
      ],
      'no-restricted-syntax': [
        'error',
        ...DYNAMIC_MAIN_IMPORT_SELECTORS.map((selector) => ({
          selector,
          message: SHARED_ISOLATION_MESSAGE,
        })),
      ],
    },
  },

  prettier,
)
