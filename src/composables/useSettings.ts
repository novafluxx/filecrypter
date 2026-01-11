// composables/useSettings.ts - Application Settings Management
//
// This composable provides persistent settings storage using Tauri Store Plugin.
// Settings are automatically saved to a JSON file in the app data directory.

import { ref, readonly, watch } from 'vue';
import { Store } from '@tauri-apps/plugin-store';

/**
 * Available theme modes
 */
export type ThemeMode = 'light' | 'dark' | 'system';

/**
 * Application settings structure
 */
export interface AppSettings {
  theme: ThemeMode;
  defaultCompression: boolean;
  defaultNeverOverwrite: boolean;
  defaultOutputDirectory: string | null;
}

/** Default settings values */
const DEFAULTS: AppSettings = {
  theme: 'system',
  defaultCompression: false,
  defaultNeverOverwrite: true,
  defaultOutputDirectory: null,
};

/** Legacy localStorage key for theme migration */
const LEGACY_THEME_KEY = 'filecrypter-theme';

// Singleton store instance
let store: Store | null = null;
let initPromise: Promise<void> | null = null;

// Reactive settings state (shared across all composable instances)
const theme = ref<ThemeMode>(DEFAULTS.theme);
const defaultCompression = ref<boolean>(DEFAULTS.defaultCompression);
const defaultNeverOverwrite = ref<boolean>(DEFAULTS.defaultNeverOverwrite);
const defaultOutputDirectory = ref<string | null>(DEFAULTS.defaultOutputDirectory);
const isInitialized = ref(false);

/**
 * Migrate legacy theme preference from localStorage
 * Only runs once on first initialization
 */
async function migrateLegacyTheme(): Promise<ThemeMode | null> {
  const legacyTheme = localStorage.getItem(LEGACY_THEME_KEY);
  if (legacyTheme === 'light' || legacyTheme === 'dark') {
    // Remove legacy key after reading
    localStorage.removeItem(LEGACY_THEME_KEY);
    return legacyTheme;
  }
  return null;
}

/**
 * Initialize the settings store
 * This is called automatically on first use
 */
async function initializeStore(): Promise<void> {
  if (store) return;

  store = await Store.load('settings.json', {
    autoSave: 100,
    defaults: { ...DEFAULTS },
  });

  // Check for legacy theme migration
  const legacyTheme = await migrateLegacyTheme();

  // Load each setting from store or use defaults
  const storedTheme = await store.get<ThemeMode>('theme');
  const storedCompression = await store.get<boolean>('defaultCompression');
  const storedOverwrite = await store.get<boolean>('defaultNeverOverwrite');
  const storedOutputDir = await store.get<string | null>('defaultOutputDirectory');

  // Apply settings with migration fallback
  theme.value = legacyTheme ?? storedTheme ?? DEFAULTS.theme;
  defaultCompression.value = storedCompression ?? DEFAULTS.defaultCompression;
  defaultNeverOverwrite.value = storedOverwrite ?? DEFAULTS.defaultNeverOverwrite;
  defaultOutputDirectory.value = storedOutputDir ?? DEFAULTS.defaultOutputDirectory;

  // If we migrated a legacy theme, save it to new store
  if (legacyTheme) {
    await store.set('theme', legacyTheme);
  }

  isInitialized.value = true;
}

/**
 * Ensure store is initialized (singleton pattern)
 */
function ensureInitialized(): Promise<void> {
  if (!initPromise) {
    initPromise = initializeStore();
  }
  return initPromise;
}

/**
 * Composable for managing application settings
 *
 * Settings are persisted automatically via Tauri Store Plugin.
 * All instances share the same reactive state.
 *
 * @example
 * ```ts
 * const { theme, setTheme, defaultCompression } = useSettings();
 *
 * // Initialize on app startup
 * await initSettings();
 *
 * // Change a setting (auto-saved)
 * setTheme('dark');
 * ```
 */
export function useSettings() {
  /**
   * Initialize settings store
   * Should be called once at app startup
   */
  async function initSettings(): Promise<void> {
    await ensureInitialized();
  }

  /**
   * Set theme preference
   */
  async function setTheme(newTheme: ThemeMode): Promise<void> {
    await ensureInitialized();
    theme.value = newTheme;
    await store?.set('theme', newTheme);
  }

  /**
   * Set default compression preference
   */
  async function setDefaultCompression(enabled: boolean): Promise<void> {
    await ensureInitialized();
    defaultCompression.value = enabled;
    await store?.set('defaultCompression', enabled);
  }

  /**
   * Set default never-overwrite preference
   */
  async function setDefaultNeverOverwrite(enabled: boolean): Promise<void> {
    await ensureInitialized();
    defaultNeverOverwrite.value = enabled;
    await store?.set('defaultNeverOverwrite', enabled);
  }

  /**
   * Set default output directory
   */
  async function setDefaultOutputDirectory(path: string | null): Promise<void> {
    await ensureInitialized();
    defaultOutputDirectory.value = path;
    await store?.set('defaultOutputDirectory', path);
  }

  /**
   * Reset all settings to defaults
   */
  async function resetToDefaults(): Promise<void> {
    await ensureInitialized();

    theme.value = DEFAULTS.theme;
    defaultCompression.value = DEFAULTS.defaultCompression;
    defaultNeverOverwrite.value = DEFAULTS.defaultNeverOverwrite;
    defaultOutputDirectory.value = DEFAULTS.defaultOutputDirectory;

    await store?.set('theme', DEFAULTS.theme);
    await store?.set('defaultCompression', DEFAULTS.defaultCompression);
    await store?.set('defaultNeverOverwrite', DEFAULTS.defaultNeverOverwrite);
    await store?.set('defaultOutputDirectory', DEFAULTS.defaultOutputDirectory);
  }

  /**
   * Get all current settings as an object
   */
  function getAllSettings(): AppSettings {
    return {
      theme: theme.value,
      defaultCompression: defaultCompression.value,
      defaultNeverOverwrite: defaultNeverOverwrite.value,
      defaultOutputDirectory: defaultOutputDirectory.value,
    };
  }

  return {
    // State (readonly refs to prevent direct mutation)
    theme: readonly(theme),
    defaultCompression: readonly(defaultCompression),
    defaultNeverOverwrite: readonly(defaultNeverOverwrite),
    defaultOutputDirectory: readonly(defaultOutputDirectory),
    isInitialized: readonly(isInitialized),

    // Methods
    initSettings,
    setTheme,
    setDefaultCompression,
    setDefaultNeverOverwrite,
    setDefaultOutputDirectory,
    resetToDefaults,
    getAllSettings,
  };
}
