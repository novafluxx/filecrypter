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
import { onMounted, watch } from 'vue';
import { useFileOps } from '../composables/useFileOps';
import { useTauri } from '../composables/useTauri';
import { useProgress } from '../composables/useProgress';
import { useDragDrop } from '../composables/useDragDrop';
import { usePasswordVisibility } from '../composables/usePasswordVisibility';
import { useSettings } from '../composables/useSettings';
import ProgressBar from './ProgressBar.vue';
import IconEye from './icons/IconEye.vue';
import IconEyeOff from './icons/IconEyeOff.vue';

// Initialize composables
const fileOps = useFileOps();
const tauri = useTauri();
const settings = useSettings();

// Apply default settings when initialized
watch(
  () => settings.isInitialized.value,
  (initialized) => {
    if (initialized) {
      fileOps.neverOverwrite.value = settings.defaultNeverOverwrite.value;
    }
  },
  { immediate: true }
);

/**
 * Get suggested output path considering default output directory
 */
function getSuggestedOutputPath(inputPath: string): string {
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
    return `${defaultDir}/${outputFilename}`;
  }

  // Use same directory as input file
  const inputDir = inputPath.substring(0, inputPath.lastIndexOf('/') + 1) ||
                   inputPath.substring(0, inputPath.lastIndexOf('\\') + 1);
  return inputDir + outputFilename;
}

// Password visibility toggle
const { isPasswordVisible, togglePasswordVisibility } = usePasswordVisibility();

// Progress tracking for decryption operation
const { progress, isActive: showProgress, startListening, stopListening } = useProgress();

// Drag-and-drop file handling
const { isDragging, handleDragOver, handleDragLeave, handleDrop, setupDragDrop } = useDragDrop(
  (path) => {
    fileOps.setInputPath(path, false); // false = decryption mode

    // If a default output directory is set, use it
    const defaultDir = settings.defaultOutputDirectory.value;
    if (defaultDir) {
      fileOps.setOutputPath(getSuggestedOutputPath(path));
    }
  }
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
      fileOps.setOutputPath(getSuggestedOutputPath(path));
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
        <input
          id="decrypt-input"
          type="text"
          :value="fileOps.inputPath.value"
          readonly
          placeholder="Select or drag a .encrypted file..."
          class="file-input"
          title="Drag a .encrypted file here or click Browse to select one"
        />
        <button
          @click="handleSelectFile"
          class="btn btn-primary"
          :disabled="fileOps.isProcessing.value"
          title="Choose an encrypted file to decrypt"
        >
          Browse
        </button>
      </div>
    </div>

    <!-- Output Path Section -->
    <div class="form-group">
      <label for="decrypt-output">Save Decrypted File As:</label>
      <div class="file-input-group">
        <input
          id="decrypt-output"
          type="text"
          :value="fileOps.outputPath.value"
          readonly
          placeholder="Will auto-generate from encrypted filename..."
          class="file-input"
          title="Auto-generated output path; click Change to pick a different location"
        />
        <button
          @click="handleSelectOutput"
          class="btn btn-secondary"
          :disabled="fileOps.isProcessing.value"
          title="Choose where to save the decrypted file"
        >
          Change
        </button>
      </div>
    </div>

    <!-- Output Safety Options -->
    <div class="form-group">
      <label class="checkbox-row">
        <input
          type="checkbox"
          v-model="fileOps.neverOverwrite.value"
          :disabled="fileOps.isProcessing.value"
          title="Prevent overwriting by auto-renaming on name conflicts"
        />
        Never overwrite existing files (auto-rename on conflicts)
      </label>
      <p class="hint-text">
        If the output name already exists, we'll save as "name (1)".
      </p>
    </div>

    <!-- Password Input Section -->
    <div class="form-group password-section">
      <label for="decrypt-password">Password:</label>
      <div class="password-input-wrapper">
        <input
          id="decrypt-password"
          :type="isPasswordVisible ? 'text' : 'password'"
          :value="fileOps.password.value"
          @input="fileOps.setPassword(($event.target as HTMLInputElement).value)"
          placeholder="Enter decryption password"
          autocomplete="current-password"
          class="password-input"
          :disabled="fileOps.isProcessing.value"
          title="Enter the password used to encrypt this file"
        />
        <button
          type="button"
          class="password-toggle-btn"
          @click="togglePasswordVisibility"
          :disabled="fileOps.isProcessing.value"
          :aria-label="isPasswordVisible ? 'Hide password' : 'Show password'"
          :title="isPasswordVisible ? 'Hide password' : 'Show password'"
        >
          <IconEye v-if="!isPasswordVisible" />
          <IconEyeOff v-else />
        </button>
      </div>
      <!-- Info hint -->
      <p v-if="fileOps.password.value.length === 0" class="hint-text">
        Enter the password used to encrypt this file
      </p>
    </div>

    <!-- Decrypt Button -->
    <button
      @click="handleDecrypt"
      class="btn btn-action"
      :disabled="!fileOps.isFormValid.value"
      title="Start decrypting with the selected file and password"
    >
      <span v-if="fileOps.isProcessing.value">Decrypting...</span>
      <span v-else>Decrypt File</span>
    </button>

    <!-- Progress Bar (shown during decryption) -->
    <ProgressBar
      v-if="showProgress && progress"
      :percent="progress.percent"
      :message="progress.message"
    />

      <!-- Status Message -->
      <div
        v-if="fileOps.statusMessage.value"
        class="status-message"
        :class="`status-${fileOps.statusType.value}`"
        role="status"
        aria-live="polite"
      >
        {{ fileOps.statusMessage.value }}
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Component-specific styles - shared styles are in src/shared.css */

.tab-content {
  padding: 16px;
  max-width: 800px;
  margin: 0 auto;
  position: relative;
}

.content-panel {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px;
  position: relative;
}
</style>
