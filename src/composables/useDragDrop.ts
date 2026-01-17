// composables/useDragDrop.ts - Drag and Drop File Handling
//
// This composable provides drag-and-drop functionality for file selection.
// It uses Tauri's native drag-and-drop event for accessing file paths.
//
// Tauri Drag-Drop:
// - Web File API doesn't expose full file paths (security restriction)
// - Tauri's onDragDropEvent provides native file paths
// - We listen to window-level drag events for visual feedback

import { ref, onUnmounted } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';

/**
 * Composable for handling file drag-and-drop
 *
 * @param onFileDrop - Callback when file is dropped
 * @returns Object containing drag state and event handlers
 *
 * @example
 * ```ts
 * const { isDragging, setupDragDrop } = useDragDrop((path) => {
 *   fileOps.setInputPath(path, true);
 * });
 *
 * onMounted(() => setupDragDrop());
 * ```
 */
export function useDragDrop(onFileDrop: (path: string) => void) {
  /** Whether a file is currently being dragged over the drop zone */
  const isDragging = ref(false);

  /** Cleanup function for Tauri event listener */
  let unlistenDrop: (() => void) | null = null;

  /**
   * Handle dragover event (for visual feedback)
   * Must call preventDefault to allow drop
   */
  function handleDragOver(event: DragEvent) {
    event.preventDefault();
    event.stopPropagation();
    isDragging.value = true;
  }

  /**
   * Handle dragleave event
   */
  function handleDragLeave(event: DragEvent) {
    event.preventDefault();
    event.stopPropagation();
    isDragging.value = false;
  }

  /**
   * Handle drop event (web API - for visual feedback only)
   * Actual file path comes from Tauri's onDragDropEvent
   */
  function handleDrop(event: DragEvent) {
    event.preventDefault();
    event.stopPropagation();
    isDragging.value = false;
    // File path handling is done by Tauri's onDragDropEvent
  }

  /**
   * Setup Tauri drag-drop event listener
   *
   * Call this in onMounted or when the component is ready.
   * This uses Tauri's native drag-drop event to get file paths.
   */
  async function setupDragDrop() {
    try {
      const appWindow = getCurrentWindow();

      // Listen for native file drops
      unlistenDrop = await appWindow.onDragDropEvent((event) => {
        if (event.payload.type === 'over' || event.payload.type === 'enter') {
          isDragging.value = true;
        } else if (event.payload.type === 'leave') {
          isDragging.value = false;
        } else if (event.payload.type === 'drop') {
          isDragging.value = false;

          // Get the dropped file paths
          const paths = event.payload.paths;
          const firstPath = paths[0];
          if (firstPath) {
            // Use the first dropped file
            onFileDrop(firstPath);
          }
        }
      });
    } catch (error) {
      console.error('Failed to setup drag-drop listener:', error);
    }
  }

  /**
   * Cleanup drag-drop event listener
   */
  function cleanupDragDrop() {
    if (unlistenDrop) {
      unlistenDrop();
      unlistenDrop = null;
    }
  }

  // Auto-cleanup on component unmount
  onUnmounted(() => {
    cleanupDragDrop();
  });

  return {
    isDragging,
    handleDragOver,
    handleDragLeave,
    handleDrop,
    setupDragDrop,
    cleanupDragDrop,
  };
}
