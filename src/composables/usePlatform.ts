// usePlatform.ts - Platform detection composable
//
// Detects whether the app is running on mobile (iOS/Android) or desktop.
// Uses Tauri's OS plugin for reliable platform detection.

import { ref, onMounted } from 'vue';
import { platform } from '@tauri-apps/plugin-os';

// Cached platform state (shared across all component instances)
const isMobile = ref(false);
const platformName = ref<string>('');
const isInitialized = ref(false);

/**
 * Composable for platform detection
 *
 * @returns Object with platform detection state
 */
export function usePlatform() {
  onMounted(async () => {
    // Only initialize once
    if (isInitialized.value) return;

    try {
      const currentPlatform = await platform();
      platformName.value = currentPlatform;
      isMobile.value = ['ios', 'android'].includes(currentPlatform);
      isInitialized.value = true;
    } catch (error) {
      console.error('Failed to detect platform:', error);
      // Default to desktop on error
      isMobile.value = false;
      isInitialized.value = true;
    }
  });

  return {
    isMobile,
    platformName,
    isInitialized,
  };
}
