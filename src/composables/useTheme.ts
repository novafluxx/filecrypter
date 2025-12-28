// composables/useTheme.ts - Theme Management
//
// This composable provides dark/light theme switching functionality.
// Theme preference is persisted to localStorage and respects system preference.
//
// CSS Variables Approach:
// - Define CSS variables in :root for light theme
// - Override in [data-theme="dark"] for dark theme
// - Components use var(--*) for colors

import { ref, watch, onMounted } from 'vue';

/**
 * Available theme modes
 */
export type Theme = 'light' | 'dark';

/** localStorage key for persisting theme preference */
const STORAGE_KEY = 'filecrypter-theme';

/**
 * Composable for managing application theme
 *
 * @returns Object containing theme state and control functions
 *
 * @example
 * ```ts
 * const { theme, toggleTheme } = useTheme();
 *
 * // In template:
 * // <button @click="toggleTheme">{{ theme === 'light' ? 'Dark' : 'Light' }}</button>
 * ```
 */
export function useTheme() {
  /** Current theme */
  const theme = ref<Theme>('light');

  /**
   * Apply theme to document
   *
   * Sets the data-theme attribute on document element,
   * which triggers CSS variable changes.
   *
   * @param newTheme - Theme to apply
   */
  function setTheme(newTheme: Theme) {
    theme.value = newTheme;
    document.documentElement.setAttribute('data-theme', newTheme);
    localStorage.setItem(STORAGE_KEY, newTheme);
  }

  /**
   * Toggle between light and dark themes
   */
  function toggleTheme() {
    setTheme(theme.value === 'light' ? 'dark' : 'light');
  }

  /**
   * Initialize theme from stored preference or system preference
   */
  function initTheme() {
    // Check localStorage first
    const storedTheme = localStorage.getItem(STORAGE_KEY) as Theme | null;

    if (storedTheme && (storedTheme === 'light' || storedTheme === 'dark')) {
      setTheme(storedTheme);
      return;
    }

    // Fall back to system preference
    if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
      setTheme('dark');
    } else {
      setTheme('light');
    }
  }

  // Initialize on mount
  onMounted(() => {
    initTheme();

    // Listen for system theme changes
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleChange = (e: MediaQueryListEvent) => {
      // Only auto-switch if user hasn't manually set a preference
      if (!localStorage.getItem(STORAGE_KEY)) {
        setTheme(e.matches ? 'dark' : 'light');
      }
    };

    mediaQuery.addEventListener('change', handleChange);
  });

  return {
    theme,
    setTheme,
    toggleTheme,
  };
}
