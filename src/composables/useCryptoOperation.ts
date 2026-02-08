// composables/useCryptoOperation.ts - Unified Crypto Operation Logic
//
// This composable unifies the common logic between encrypt and decrypt operations.
// It provides a single interface for setting up file operations with mode-specific
// behavior determined by the 'mode' option.

import { ref, computed, onMounted, type Ref, type ComputedRef } from 'vue';
import { join, dirname } from '@tauri-apps/api/path';
import { useFileOps } from './useFileOps';
import { useTauri } from './useTauri';
import { useProgress, type ProgressEvent } from './useProgress';
import { useDragDrop } from './useDragDrop';
import { useSettings } from './useSettings';
import { useSettingsSync } from './useSettingsSync';

export interface UseCryptoOperationOptions {
  mode: 'encrypt' | 'decrypt';
}

export interface UseCryptoOperationReturn {
  // Core state from useFileOps
  fileOps: ReturnType<typeof useFileOps>;

  // Progress state
  progress: Ref<ProgressEvent | null>;
  showProgress: Ref<boolean>;

  // Drag-drop state
  isDragging: Ref<boolean>;
  dropZoneRef: Ref<HTMLElement | undefined>;

  // Settings
  settings: ReturnType<typeof useSettings>;

  // Computed for form validation
  isFormValid: ComputedRef<boolean>;

  // Unified handlers
  handleSelectFile: () => Promise<void>;
  handleSelectOutput: () => Promise<void>;
  handleOperation: () => Promise<void>;

  // Drag event handlers (for template binding)
  handleDragOver: (e: DragEvent) => void;
  handleDragLeave: (e: DragEvent) => void;
  handleDrop: (e: DragEvent) => void;
}

/**
 * Unified composable for encrypt/decrypt operations
 *
 * Combines useFileOps, useTauri, useProgress, useDragDrop, and useSettings
 * into a single interface with mode-specific behavior.
 */
export function useCryptoOperation(options: UseCryptoOperationOptions): UseCryptoOperationReturn {
  const { mode } = options;
  const isEncrypt = mode === 'encrypt';

  // Initialize composables
  const fileOps = useFileOps();
  const tauri = useTauri();
  const settings = useSettings();

  // Sync settings to fileOps state
  if (isEncrypt) {
    useSettingsSync(settings, {
      compression: fileOps.compressionEnabled,
      neverOverwrite: fileOps.neverOverwrite,
    });
  } else {
    useSettingsSync(settings, {
      neverOverwrite: fileOps.neverOverwrite,
    });
  }

  // Progress tracking
  const { progress, isActive: showProgress, startListening, stopListening } = useProgress();

  // Drop zone element reference
  const dropZoneRef = ref<HTMLElement>();

  /**
   * Get suggested output path for decrypt mode considering default output directory
   */
  async function getSuggestedDecryptOutputPath(inputPath: string): Promise<string> {
    const defaultDir = settings.defaultOutputDirectory.value;

    // Extract filename from input path
    const filename = inputPath.split(/[/\\]/).pop() ?? '';

    // Determine output filename (remove .encrypted or add .decrypted)
    let outputFilename: string;
    if (filename.endsWith('.encrypted')) {
      outputFilename = filename.slice(0, -'.encrypted'.length);
    } else {
      outputFilename = filename + '.decrypted';
    }

    if (defaultDir) {
      return await join(defaultDir, outputFilename);
    }

    // Use same directory as input file
    const inputDir = await dirname(inputPath);
    return await join(inputDir, outputFilename);
  }

  // Drag-and-drop file handling
  const { isDragging, handleDragOver, handleDragLeave, handleDrop, setupDragDrop } = useDragDrop(
    async (paths) => {
      const path = paths[0];
      if (path) {
        fileOps.setInputPath(path, isEncrypt);

        // Apply default output directory if set
        const defaultDir = settings.defaultOutputDirectory.value;
        if (defaultDir) {
          if (isEncrypt) {
            const filename = path.split(/[/\\]/).pop() ?? '';
            const outputPath = await join(defaultDir, `${filename}.encrypted`);
            fileOps.setOutputPath(outputPath);
          } else {
            const outputPath = await getSuggestedDecryptOutputPath(path);
            fileOps.setOutputPath(outputPath);
          }
        }
      }
    },
    dropZoneRef
  );

  // Setup drag-drop on mount
  onMounted(() => {
    setupDragDrop();
  });

  // Form validation (mode-specific)
  const isFormValid = computed(() =>
    isEncrypt ? fileOps.isEncryptFormValid.value : fileOps.isDecryptFormValid.value
  );

  /**
   * Handle file selection
   */
  async function handleSelectFile() {
    const dialogTitle = isEncrypt ? 'Select File to Encrypt' : 'Select Encrypted File';
    const filters = isEncrypt
      ? undefined
      : [
          { name: 'Encrypted Files', extensions: ['encrypted'] },
          { name: 'All Files', extensions: ['*'] },
        ];

    const path = await tauri.selectFile(dialogTitle, filters);

    if (path) {
      fileOps.setInputPath(path, isEncrypt);

      // Apply default output directory if set
      const defaultDir = settings.defaultOutputDirectory.value;
      if (defaultDir) {
        if (isEncrypt) {
          const filename = path.split(/[/\\]/).pop() ?? '';
          const outputPath = await join(defaultDir, `${filename}.encrypted`);
          fileOps.setOutputPath(outputPath);
        } else {
          const outputPath = await getSuggestedDecryptOutputPath(path);
          fileOps.setOutputPath(outputPath);
        }
      }
    }
  }

  /**
   * Handle output path selection
   */
  async function handleSelectOutput() {
    const dialogTitle = isEncrypt ? 'Save Encrypted File As' : 'Save Decrypted File As';
    const filters = isEncrypt ? [{ name: 'Encrypted Files', extensions: ['encrypted'] }] : undefined;

    const path = await tauri.selectSavePath(dialogTitle, fileOps.outputPath.value, filters);

    if (path) {
      fileOps.setOutputPath(path);
    }
  }

  /**
   * Handle the main operation (encrypt or decrypt)
   */
  async function handleOperation() {
    await startListening();

    let success = false;
    try {
      success = isEncrypt ? await fileOps.performEncrypt() : await fileOps.performDecrypt();
    } finally {
      if (!success) {
        stopListening();
      }
    }
  }

  return {
    fileOps,
    progress,
    showProgress,
    isDragging,
    dropZoneRef,
    settings,
    isFormValid,
    handleSelectFile,
    handleSelectOutput,
    handleOperation,
    handleDragOver,
    handleDragLeave,
    handleDrop,
  };
}
