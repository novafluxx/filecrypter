// composables/useProgress.ts - Progress Event Listener
//
// This composable listens for progress events from the Rust backend
// and provides reactive state for displaying progress in the UI.
//
// Tauri Event System:
// - Backend emits events using app.emit("crypto-progress", payload)
// - Frontend listens using listen() from @tauri-apps/api/event
// - Events are typed and parsed from JSON automatically

import { ref, onUnmounted } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

/**
 * Progress event payload from Rust backend
 */
export interface ProgressEvent {
  /** Current stage: "reading", "deriving_key", "encrypting", "decrypting", "writing", "complete" */
  stage: string;
  /** Progress percentage (0-100) */
  percent: number;
  /** Human-readable status message */
  message: string;
}

/** Event name constant - must match CRYPTO_PROGRESS_EVENT in Rust */
const CRYPTO_PROGRESS_EVENT = 'crypto-progress';

/**
 * Composable for listening to crypto progress events
 *
 * @returns Object containing progress state and control functions
 *
 * @example
 * ```ts
 * const { progress, isActive, startListening, stopListening } = useProgress();
 *
 * // Before starting encryption:
 * await startListening();
 *
 * // In template:
 * // <ProgressBar v-if="isActive" :percent="progress?.percent" :message="progress?.message" />
 * ```
 */
export function useProgress() {
  /** Current progress event (null when no operation in progress) */
  const progress = ref<ProgressEvent | null>(null);

  /** Whether progress tracking is active */
  const isActive = ref(false);

  /** Unlisten function to clean up event listener */
  let unlisten: UnlistenFn | null = null;

  /** Operation counter to prevent race conditions with stale timeouts */
  let operationId = 0;

  /**
   * Start listening for progress events
   *
   * Call this before initiating an encrypt/decrypt operation.
   * The listener will automatically track progress until completion.
   */
  async function startListening() {
    // Clean up any existing listener first (prevents leak with KeepAlive)
    if (unlisten) {
      unlisten();
      unlisten = null;
    }

    // Track this operation to handle race conditions
    operationId++;
    const currentOpId = operationId;

    // Reset state
    isActive.value = true;
    progress.value = null;

    try {
      // Register event listener
      unlisten = await listen<ProgressEvent>(CRYPTO_PROGRESS_EVENT, (event) => {
        // Ignore events from stale operations
        if (currentOpId !== operationId) return;

        progress.value = event.payload;

        // Auto-deactivate when operation completes
        if (event.payload.stage === 'complete') {
          // Keep showing for a moment so user sees 100%
          setTimeout(() => {
            // Only deactivate if this is still the current operation
            if (currentOpId === operationId) {
              isActive.value = false;
              if (unlisten) {
                unlisten();
                unlisten = null;
              }
            }
          }, 500);
        }
      });
    } catch (error) {
      console.error('Failed to setup progress listener:', error);
      isActive.value = false;
    }
  }

  /**
   * Stop listening for progress events
   *
   * Call this to clean up the listener early (e.g., on error or cancel)
   */
  function stopListening() {
    if (unlisten) {
      unlisten();
      unlisten = null;
    }
    isActive.value = false;
    progress.value = null;
  }

  // Clean up listener when component unmounts
  onUnmounted(() => {
    if (unlisten) {
      unlisten();
      unlisten = null;
    }
  });

  return {
    progress,
    isActive,
    startListening,
    stopListening,
  };
}
