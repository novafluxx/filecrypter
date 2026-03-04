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

import { ref } from 'vue';
import { platform } from '@tauri-apps/plugin-os';

// Global state (shared across all component instances using this composable)
// This ensures platform detection only happens once, regardless of how many
// components use usePlatform()
const isMobile = ref(false);
const isInitialized = ref(false);

// Run detection eagerly at module load time (once, not per-component-mount)
try {
  const currentPlatform = platform();
  isMobile.value = ['ios', 'android'].includes(currentPlatform);
} catch (error) {
  console.error('Failed to detect platform:', error);
  isMobile.value = false;
}
isInitialized.value = true;

/**
 * Composable for detecting the current platform (mobile vs desktop).
 *
 * State is initialized eagerly when the module is first imported.
 * All calls return the same shared reactive refs — no hook registration.
 */
export function usePlatform() {
  return {
    /** True if running on iOS or Android */
    isMobile,
    /** True once platform detection has completed */
    isInitialized,
  };
}
