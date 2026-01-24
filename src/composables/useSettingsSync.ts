// composables/useSettingsSync.ts - Settings Synchronization
//
// This composable provides a declarative way to sync global settings
// to local component state or composable refs.
//
// Features:
// - Initial sync when settings are first initialized
// - Reactive updates when settings change in the Settings tab
// - Type-safe mapping between settings and target refs

import { type Ref, watch, type WatchStopHandle } from 'vue';
import { useSettings } from './useSettings';

/**
 * Mapping of setting keys to their corresponding target refs.
 * All properties are optional - sync only what you need.
 */
export interface SettingsSyncTargets {
  /** Sync defaultCompression to this boolean ref */
  compression?: Ref<boolean>;
  /** Sync defaultNeverOverwrite to this boolean ref */
  neverOverwrite?: Ref<boolean>;
  /** Sync defaultOutputDirectory to this string ref (empty string when cleared) */
  outputDirectory?: Ref<string>;
}

type Settings = ReturnType<typeof useSettings>;

/**
 * Creates a watcher that syncs a setting value to a target ref.
 * Only syncs when settings are initialized.
 */
function createSettingWatcher<T>(
  source: () => T,
  target: Ref<T>,
  isInitialized: Readonly<Ref<boolean>>
): WatchStopHandle {
  return watch(source, (newValue) => {
    if (isInitialized.value) {
      target.value = newValue;
    }
  });
}

/**
 * Composable for syncing settings to target refs.
 *
 * Handles both initial sync when settings are first initialized
 * and reactive updates when settings change in the Settings tab.
 *
 * @param settings - Settings composable instance from useSettings()
 * @param targets - Object mapping setting names to target refs
 * @returns Object with stop function to clean up watchers
 *
 * @example
 * ```ts
 * // In EncryptTab.vue
 * const fileOps = useFileOps();
 * const settings = useSettings();
 *
 * useSettingsSync(settings, {
 *   compression: fileOps.compressionEnabled,
 *   neverOverwrite: fileOps.neverOverwrite,
 * });
 * ```
 *
 * @example
 * ```ts
 * // In BatchTab.vue with local refs
 * const neverOverwrite = ref(true);
 * const outputDir = ref('');
 * const settings = useSettings();
 *
 * useSettingsSync(settings, {
 *   neverOverwrite,
 *   outputDirectory: outputDir,
 * });
 * ```
 */
export function useSettingsSync(
  settings: Settings,
  targets: SettingsSyncTargets
): { stop: () => void } {
  const stopHandles: WatchStopHandle[] = [];

  /**
   * Sync all targeted settings at once.
   * Used for initial sync when settings become initialized.
   */
  function syncAllSettings() {
    if (targets.compression !== undefined) {
      targets.compression.value = settings.defaultCompression.value;
    }
    if (targets.neverOverwrite !== undefined) {
      targets.neverOverwrite.value = settings.defaultNeverOverwrite.value;
    }
    // Sync outputDirectory (empty string when null/cleared)
    if (targets.outputDirectory !== undefined) {
      targets.outputDirectory.value = settings.defaultOutputDirectory.value ?? '';
    }
  }

  // Initial sync: Wait for settings to be initialized
  const initWatcher = watch(
    () => settings.isInitialized.value,
    (initialized) => {
      if (initialized) {
        syncAllSettings();
      }
    },
    { immediate: true }
  );
  stopHandles.push(initWatcher);

  // Reactive sync for compression
  if (targets.compression !== undefined) {
    const handle = createSettingWatcher(
      () => settings.defaultCompression.value,
      targets.compression,
      settings.isInitialized
    );
    stopHandles.push(handle);
  }

  // Reactive sync for neverOverwrite
  if (targets.neverOverwrite !== undefined) {
    const handle = createSettingWatcher(
      () => settings.defaultNeverOverwrite.value,
      targets.neverOverwrite,
      settings.isInitialized
    );
    stopHandles.push(handle);
  }

  // Reactive sync for outputDirectory (clears to empty string when null)
  if (targets.outputDirectory !== undefined) {
    const handle = watch(
      () => settings.defaultOutputDirectory.value,
      (newValue) => {
        if (settings.isInitialized.value) {
          targets.outputDirectory!.value = newValue ?? '';
        }
      }
    );
    stopHandles.push(handle);
  }

  return {
    stop: () => stopHandles.forEach((handle) => handle()),
  };
}
