import js from '@eslint/js'
import globals from 'globals'
import reactHooks from 'eslint-plugin-react-hooks'
import { defineConfig, globalIgnores } from 'eslint/config'

// The packages' lint (the root's `npm run lint`): what the apps' lint holds their code to, for the code they
// share — an undefined name in a package otherwise shows only when a page reaches it.

// ESLint's own no-unused-vars does not see a component used only as JSX (`<Panel />`) — as the apps' configs.
const jsxUsesVars = {
  meta: { type: 'problem', schema: [] },
  create(context) {
    return {
      JSXOpeningElement(node) {
        let name = node.name
        while (name.type === 'JSXMemberExpression') name = name.object
        if (name.type !== 'JSXIdentifier') return
        if (node.name.type === 'JSXIdentifier' && !/^[A-Z]/.test(name.name)) return
        context.sourceCode.markVariableAsUsed(name.name, node)
      },
    }
  },
}

export default defineConfig([
  globalIgnores(['**/wasm', '**/dist']),
  {
    files: ['**/*.{js,jsx,mjs}'],
    extends: [js.configs.recommended],
    plugins: { local: { rules: { 'jsx-uses-vars': jsxUsesVars } } },
    languageOptions: {
      ecmaVersion: 'latest',
      sourceType: 'module',
      // Packages run in pages, in workers and under Node (the project server, the scripts, the tests).
      globals: { ...globals.browser, ...globals.worker, ...globals.node },
      parserOptions: { ecmaFeatures: { jsx: true } },
    },
    rules: {
      'local/jsx-uses-vars': 'error',
      'no-unused-vars': ['error', { varsIgnorePattern: '^_', ignoreRestSiblings: true }],
    },
  },
  // The packages with React components; elsewhere a use… function is no hook (the module loaders).
  {
    files: ['ui/**/*.{js,jsx}', 'vehicles/**/*.{js,jsx}'],
    extends: [reactHooks.configs.flat.recommended],
  },
])
