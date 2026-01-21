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
 * @param onFileDrop - Callback when file(s) are dropped (receives array of paths)
 * @param elementRef - Optional ref to the drop zone element (to check if drop is within this element)
 * @returns Object containing drag state and event handlers
 *
 * @example
 * ```ts
 * const dropZoneRef = ref<HTMLElement>();
 * const { isDragging, setupDragDrop } = useDragDrop((paths) => {
 *   // Handle single file: use paths[0]
 *   // Handle multiple files: iterate over paths
 * }, dropZoneRef);
 *
 * onMounted(() => setupDragDrop());
 * ```
 */
export function useDragDrop(onFileDrop: (paths: string[]) => void, elementRef?: { value: HTMLElement | undefined }) {
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
          // Only show dragging state if this is the target element
          if (elementRef?.value && isDropWithinElement(event.payload.position)) {
            isDragging.value = true;
          }
        } else if (event.payload.type === 'leave') {
          isDragging.value = false;
        } else if (event.payload.type === 'drop') {
          isDragging.value = false;

          // Only handle drop if it occurred within this element's bounds
          if (elementRef?.value && !isDropWithinElement(event.payload.position)) {
            return;
          }

          // Get the dropped file paths
          const paths = event.payload.paths;
          if (paths && paths.length > 0) {
            // Pass all dropped files to the callback
            onFileDrop(paths);
          }
        }
      });
    } catch (error) {
      console.error('Failed to setup drag-drop listener:', error);
    }
  }

  /**
   * Check if drop position is within the element bounds
   */
  function isDropWithinElement(position?: { x: number; y: number }): boolean {
    if (!elementRef?.value || !position) {
      // If no element ref provided, accept all drops (backward compatible)
      return true;
    }

    const rect = elementRef.value.getBoundingClientRect();
    return (
      position.x >= rect.left &&
      position.x <= rect.right &&
      position.y >= rect.top &&
      position.y <= rect.bottom
    );
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
