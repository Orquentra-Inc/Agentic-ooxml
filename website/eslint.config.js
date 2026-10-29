import js from '@eslint/js'
import tseslint from 'typescript-eslint'
import hooks from 'eslint-plugin-react-hooks'
import a11y from 'eslint-plugin-jsx-a11y'
import globals from 'globals'

export default tseslint.config(
  { ignores: ['dist', 'node_modules', 'shots'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  a11y.flatConfigs.recommended,
  {
    files: ['src/**/*.{ts,tsx}'],
    plugins: { 'react-hooks': hooks },
    languageOptions: { globals: globals.browser },
    rules: {
      ...hooks.configs.recommended.rules,
      // Scrollable code blocks must be keyboard focusable (axe: scrollable-region-focusable).
      'jsx-a11y/no-noninteractive-tabindex': 'off',
      // Tabs use roving tabindex on the child tabs; the tablist itself only handles bubbled keys.
      'jsx-a11y/interactive-supports-focus': 'off',
    },
  },
  { files: ['scripts/**/*.mjs', 'vite.config.ts'], languageOptions: { globals: { ...globals.node, ...globals.browser } } },
)
