// useUpdater.ts - Auto-Update Composable
//
// Provides reactive auto-update functionality for desktop platforms.
// Uses Tauri's @tauri-apps/plugin-updater for checking and installing updates.
//
// Usage:
//   const { checkForUpdates, downloadAndInstall, updateAvailable, ... } = useUpdater();
//   await checkForUpdates();
//   if (updateAvailable.value) await downloadAndInstall();
//
// Note: Updates are only available on desktop platforms (macOS, Windows, Linux).
// Mobile platforms (iOS, Android) use their respective app stores for updates.

import { ref } from 'vue';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

// Global state (shared across all component instances)
const updateAvailable = ref(false);
const updateVersion = ref<string | null>(null);
const updateNotes = ref<string | null>(null);
const isChecking = ref(false);
const isDownloading = ref(false);
const downloadProgress = ref(0);
const error = ref<string | null>(null);

// Store the update object for later installation
let pendingUpdate: Update | null = null;

/**
 * Composable for auto-update functionality.
 *
 * Provides methods to check for updates, download, and install them.
 * All state is reactive and shared globally across component instances.
 *
 * @returns {Object} Update state and methods
 */
export function useUpdater() {
  /**
   * Check for available updates.
   *
   * @returns {Promise<boolean>} True if an update is available
   */
  async function checkForUpdates(): Promise<boolean> {
    // Reset state
    error.value = null;
    isChecking.value = true;
    updateAvailable.value = false;
    updateVersion.value = null;
    updateNotes.value = null;
    pendingUpdate = null;

    try {
      const update = await check();

      if (update) {
        pendingUpdate = update;
        updateAvailable.value = true;
        updateVersion.value = update.version;
        updateNotes.value = update.body ?? null;
        return true;
      }

      return false;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to check for updates';
      error.value = message;
      console.error('Update check failed:', err);
      return false;
    } finally {
      isChecking.value = false;
    }
  }

  /**
   * Download and install the pending update.
   *
   * This will download the update, install it, and relaunch the app.
   * Progress is tracked via the downloadProgress ref.
   */
  async function downloadAndInstall(): Promise<void> {
    if (!pendingUpdate) {
      error.value = 'No update available to install';
      return;
    }

    error.value = null;
    isDownloading.value = true;
    downloadProgress.value = 0;

    try {
      // Track total bytes for progress calculation
      let totalBytes = 0;
      let downloadedBytes = 0;

      // Download and install with progress tracking
      await pendingUpdate.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            downloadProgress.value = 0;
            downloadedBytes = 0;
            totalBytes = event.data.contentLength ?? 0;
            break;
          case 'Progress':
            downloadedBytes += event.data.chunkLength;
            if (totalBytes > 0) {
              downloadProgress.value = Math.round((downloadedBytes / totalBytes) * 100);
            }
            break;
          case 'Finished':
            downloadProgress.value = 100;
            break;
        }
      });

      // Relaunch the app to apply the update
      await relaunch();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to install update';
      error.value = message;
      console.error('Update installation failed:', err);
    } finally {
      isDownloading.value = false;
    }
  }

  /**
   * Dismiss the current update notification.
   * User can check again later or will be notified on next app launch.
   */
  function dismissUpdate(): void {
    updateAvailable.value = false;
    pendingUpdate = null;
  }

  return {
    /** True if an update is available */
    updateAvailable,
    /** Version string of the available update */
    updateVersion,
    /** Release notes for the available update */
    updateNotes,
    /** True while checking for updates */
    isChecking,
    /** True while downloading/installing an update */
    isDownloading,
    /** Download progress (0-100) */
    downloadProgress,
    /** Error message if update check or install failed */
    error,
    /** Check for available updates */
    checkForUpdates,
    /** Download and install the pending update */
    downloadAndInstall,
    /** Dismiss the update notification */
    dismissUpdate,
  };
}
