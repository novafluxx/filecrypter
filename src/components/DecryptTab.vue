<!-- components/DecryptTab.vue - File Decryption Interface -->
<!--
  This component provides the UI for decrypting files.

  Features:
  - File selection filtered to .encrypted files
  - Auto-removes .encrypted extension from output filename
  - Password input (must match encryption password)
  - Real-time form validation
  - Status messages showing success/wrong password/errors

  Note: The structure is very similar to EncryptTab but with
  slight differences in file filtering and output path logic.
-->

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { NButton, NInput } from 'naive-ui';
import { join, dirname } from '@tauri-apps/api/path';
import { useFileOps } from '../composables/useFileOps';
import { useTauri } from '../composables/useTauri';
import { useProgress } from '../composables/useProgress';
import { useDragDrop } from '../composables/useDragDrop';
import { useSettings } from '../composables/useSettings';
import { useSettingsSync } from '../composables/useSettingsSync';
import OverwriteCheckbox from './OverwriteCheckbox.vue';
import PasswordSection from './PasswordSection.vue';
import ProgressBar from './ProgressBar.vue';
import StatusMessage from './StatusMessage.vue';

// Initialize composables
const fileOps = useFileOps();
const tauri = useTauri();
const settings = useSettings();

// Sync settings to fileOps state (initial + reactive updates)
useSettingsSync(settings, {
  neverOverwrite: fileOps.neverOverwrite,
});

/**
 * Get suggested output path considering default output directory
 */
async function getSuggestedOutputPath(inputPath: string): Promise<string> {
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

// Progress tracking for decryption operation
const { progress, isActive: showProgress, startListening, stopListening } = useProgress();

// Drop zone element reference for drag-and-drop
const dropZoneRef = ref<HTMLElement>();

// Drag-and-drop file handling
const { isDragging, handleDragOver, handleDragLeave, handleDrop, setupDragDrop } = useDragDrop(
  async (paths) => {
    // For single file decryption, use the first dropped file
    const path = paths[0];
    if (path) {
      fileOps.setInputPath(path, false); // false = decryption mode

      // If a default output directory is set, use it
      const defaultDir = settings.defaultOutputDirectory.value;
      if (defaultDir) {
        const outputPath = await getSuggestedOutputPath(path);
        fileOps.setOutputPath(outputPath);
      }
    }
  },
  dropZoneRef
);

// Setup drag-drop on mount
onMounted(() => {
  setupDragDrop();
});

/**
 * Handle encrypted file selection
 *
 * Opens file picker filtered to .encrypted files
 */
async function handleSelectFile() {
  const path = await tauri.selectFile(
    'Select Encrypted File',
    [
      { name: 'Encrypted Files', extensions: ['encrypted'] },
      { name: 'All Files', extensions: ['*'] }
    ]
  );

  if (path) {
    // setInputPath with false = decryption mode (removes .encrypted extension)
    fileOps.setInputPath(path, false);

    // If a default output directory is set, use it
    const defaultDir = settings.defaultOutputDirectory.value;
    if (defaultDir) {
      const outputPath = await getSuggestedOutputPath(path);
      fileOps.setOutputPath(outputPath);
    }
  }
}

/**
 * Handle output path selection for decrypted file
 */
async function handleSelectOutput() {
  const path = await tauri.selectSavePath(
    'Save Decrypted File As',
    fileOps.outputPath.value
  );

  if (path) {
    fileOps.setOutputPath(path);
  }
}

/**
 * Handle decryption button click
 *
 * Calls Rust backend to decrypt the file
 * Common errors:
 * - Wrong password (authentication tag verification fails)
 * - Corrupted file (invalid format)
 * - File tampering (tag mismatch)
 */
async function handleDecrypt() {
  // Start listening for progress events before decryption begins
  await startListening();

  let success = false;
  try {
    success = await fileOps.performDecrypt();
    // Password is automatically cleared for security
  } finally {
    // On error or failure, reset progress immediately
    // On success, progress auto-resets after showing 100%
    if (!success) {
      stopListening();
    }
  }
}
</script>

<template>
  <div class="tab-content">
    <div
      ref="dropZoneRef"
      class="content-panel"
      :class="{ 'drop-zone-active': isDragging }"
      @dragover="handleDragOver"
      @dragleave="handleDragLeave"
      @drop="handleDrop"
    >
      <!-- Drop overlay (shown when dragging file over) -->
      <div v-if="isDragging" class="drop-overlay">
        Drop encrypted file here
      </div>

      <!-- Encrypted File Input Section -->
      <div class="form-group">
        <label for="decrypt-input">Encrypted File:</label>
        <div class="file-input-group">
          <NInput
            :input-props="{ id: 'decrypt-input' }"
            :value="fileOps.inputPath.value"
            readonly
            placeholder="Select or drag a .encrypted file..."
          />
          <NButton
            type="primary"
            @click="handleSelectFile"
            :disabled="fileOps.isProcessing.value"
            title="Choose an encrypted file to decrypt"
          >
            Browse
          </NButton>
        </div>
      </div>

      <!-- Output Path Section -->
      <div class="form-group">
        <label for="decrypt-output">Save Decrypted File As:</label>
        <div class="file-input-group">
          <NInput
            :input-props="{ id: 'decrypt-output' }"
            :value="fileOps.outputPath.value"
            readonly
            placeholder="Will auto-generate from encrypted filename..."
          />
          <NButton
            @click="handleSelectOutput"
            :disabled="fileOps.isProcessing.value"
            title="Choose where to save the decrypted file"
          >
            Change
          </NButton>
        </div>
      </div>

      <!-- Output Safety Options -->
      <OverwriteCheckbox
        v-model="fileOps.neverOverwrite.value"
        :disabled="fileOps.isProcessing.value"
      />

      <!-- Password Input Section -->
      <PasswordSection
        input-id="decrypt-password"
        v-model="fileOps.password.value"
        placeholder="Enter decryption password"
        :disabled="fileOps.isProcessing.value"
        hint-text="Enter the password used to encrypt this file"
      />

      <!-- Decrypt Button -->
      <NButton
        type="primary"
        block
        strong
        class="action-btn"
        @click="handleDecrypt"
        :disabled="!fileOps.isDecryptFormValid.value"
        title="Start decrypting with the selected file and password"
      >
        <span v-if="fileOps.isProcessing.value">Decrypting...</span>
        <span v-else>Decrypt File</span>
      </NButton>

      <!-- Progress Bar (shown during decryption) -->
      <ProgressBar
        v-if="showProgress && progress"
        :percent="progress.percent"
        :message="progress.message"
      />

      <!-- Status Message -->
      <StatusMessage
        v-if="fileOps.statusMessage.value"
        :message="fileOps.statusMessage.value"
        :type="fileOps.statusType.value"
      />
    </div>
  </div>
</template>

<style scoped>
/* Component-specific styles - shared styles are in src/shared.css */

.tab-content {
  padding: 16px;
  max-width: 800px;
  width: 100%;
  margin: 0 auto;
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.content-panel {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px;
  position: relative;
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
}

.action-btn {
  margin-top: 8px;
}
</style>
