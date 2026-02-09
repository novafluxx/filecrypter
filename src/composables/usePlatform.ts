// usePlatform.ts - Platform Detection Composable
//
// Provides reactive platform detection for conditional UI rendering.
// Uses Tauri's @tauri-apps/plugin-os for reliable native platform detection.
//
// Usage:
//   const { isMobile, isInitialized } = usePlatform();
//   // In template: v-if="isInitialized && isMobile" for mobile-only components
//
// Supported platforms:
//   - Desktop: 'macos', 'windows', 'linux'
//   - Mobile: 'ios', 'android'

import { ref, onMounted } from 'vue';
import { platform } from '@tauri-apps/plugin-os';

// Global state (shared across all component instances using this composable)
// This ensures platform detection only happens once, regardless of how many
// components use usePlatform()
const isMobile = ref(false);
const isInitialized = ref(false);

/**
 * Composable for detecting the current platform (mobile vs desktop).
 *
 * State is cached globally - the Tauri API is only called once on first use.
 * All subsequent calls return the cached reactive refs.
 *
 * @returns {Object} Platform detection state
 * @returns {Ref<boolean>} isMobile - True if running on iOS or Android
 * @returns {Ref<boolean>} isInitialized - True once platform detection has completed
 */
export function usePlatform() {
  onMounted(async () => {
    // Skip if already initialized (cached from previous component mount)
    if (isInitialized.value) return;

    try {
      // Call Tauri OS plugin to get the native platform
      const currentPlatform = await platform();

      // Determine if this is a mobile platform
      isMobile.value = ['ios', 'android'].includes(currentPlatform);
      isInitialized.value = true;
    } catch (error) {
      // Log error but don't crash - default to desktop behavior
      console.error('Failed to detect platform:', error);
      isMobile.value = false;
      isInitialized.value = true;
    }
  });

  return {
    /** True if running on iOS or Android */
    isMobile,
    /** True once platform detection has completed */
    isInitialized,
  };
}
