import eslint from '@eslint/js';
import tseslint from 'typescript-eslint';
import pluginVue from 'eslint-plugin-vue';

export default tseslint.config(
  eslint.configs.recommended,
  ...tseslint.configs.recommended,
  // Use 'essential' instead of 'recommended' for Vue - catches errors without style enforcement
  ...pluginVue.configs['flat/essential'],
  {
    files: ['src/**/*.vue'],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
      },
    },
  },
  {
    languageOptions: {
      globals: {
        // Browser globals
        document: 'readonly',
        window: 'readonly',
        Event: 'readonly',
        console: 'readonly',
      },
    },
  },
  {
    rules: {
      // Relax some rules for practicality
      'vue/multi-word-component-names': 'off',
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
      '@typescript-eslint/no-empty-object-type': 'off', // Allow {} in type definitions
      '@typescript-eslint/no-explicit-any': 'warn', // Warn instead of error
    },
  },
  {
    ignores: ['dist/', 'src-tauri/'],
  }
);
