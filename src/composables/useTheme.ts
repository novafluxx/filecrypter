// composables/useTheme.ts - Theme Management
//
// This composable provides dark/light/system theme switching functionality.
// Theme preference is persisted via useSettings and respects system preference.
//
// CSS Variables Approach:
// - Define CSS variables in :root for light theme
// - Override in [data-theme="dark"] for dark theme
// - Components use var(--*) for colors

import { ref, computed, watch, onMounted, onUnmounted } from 'vue';
import { useSettings, type ThemeMode } from './useSettings';

/**
 * Actual applied theme (always 'light' or 'dark')
 */
export type AppliedTheme = 'light' | 'dark';

/**
 * Composable for managing application theme
 *
 * @returns Object containing theme state and control functions
 *
 * @example
 * ```ts
 * const { theme, appliedTheme, toggleTheme, setTheme } = useTheme();
 *
 * // In template:
 * // <button @click="toggleTheme">{{ appliedTheme === 'light' ? 'Dark' : 'Light' }}</button>
 * ```
 */
export function useTheme() {
  const settings = useSettings();

  /** The actual applied theme (resolved from 'system' if needed) */
  const appliedTheme = ref<AppliedTheme>('light');

  /** Media query for system preference */
  let mediaQuery: MediaQueryList | null = null;
  let mediaQueryHandler: ((e: MediaQueryListEvent) => void) | null = null;

  /**
   * Get system preferred theme
   */
  function getSystemTheme(): AppliedTheme {
    if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
      return 'dark';
    }
    return 'light';
  }

  /**
   * Apply theme to document
   *
   * Sets the data-theme attribute on document element,
   * which triggers CSS variable changes.
   */
  function applyTheme(theme: AppliedTheme) {
    appliedTheme.value = theme;
    document.documentElement.setAttribute('data-theme', theme);
  }

  /**
   * Resolve theme mode to actual theme
   */
  function resolveTheme(mode: ThemeMode): AppliedTheme {
    if (mode === 'system') {
      return getSystemTheme();
    }
    return mode;
  }

  /**
   * Update theme based on current setting
   */
  function updateTheme() {
    const resolved = resolveTheme(settings.theme.value);
    applyTheme(resolved);
  }

  /**
   * Set theme preference (delegates to settings)
   */
  async function setTheme(newTheme: ThemeMode) {
    await settings.setTheme(newTheme);
    updateTheme();
  }

  /**
   * Toggle between light, dark, and system themes
   * Cycles: light -> dark -> system -> light
   */
  async function toggleTheme() {
    const current = settings.theme.value;
    let next: ThemeMode;

    switch (current) {
      case 'light':
        next = 'dark';
        break;
      case 'dark':
        next = 'system';
        break;
      case 'system':
      default:
        next = 'light';
        break;
    }

    await setTheme(next);
  }

  /**
   * Setup system theme change listener
   */
  function setupSystemThemeListener() {
    if (!window.matchMedia) return;

    mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    mediaQueryHandler = () => {
      // Only update if we're in system mode
      if (settings.theme.value === 'system') {
        updateTheme();
      }
    };

    mediaQuery.addEventListener('change', mediaQueryHandler);
  }

  /**
   * Cleanup system theme listener
   */
  function cleanupSystemThemeListener() {
    if (mediaQuery && mediaQueryHandler) {
      mediaQuery.removeEventListener('change', mediaQueryHandler);
      mediaQuery = null;
      mediaQueryHandler = null;
    }
  }

  // Watch for settings changes
  watch(
    () => settings.theme.value,
    () => {
      updateTheme();
    }
  );

  // Initialize on mount
  onMounted(() => {
    // Apply initial theme
    updateTheme();

    // Setup listener for system theme changes
    setupSystemThemeListener();
  });

  // Clean up on unmount
  onUnmounted(() => {
    cleanupSystemThemeListener();
  });

  return {
    /** Current theme setting (light/dark/system) */
    theme: settings.theme,
    /** Actual applied theme (light/dark) */
    appliedTheme,
    /** Set theme preference */
    setTheme,
    /** Toggle through theme options */
    toggleTheme,
  };
}
