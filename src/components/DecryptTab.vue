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
import { onMounted } from 'vue';
import { useFileOps } from '../composables/useFileOps';
import { useTauri } from '../composables/useTauri';
import { useProgress } from '../composables/useProgress';
import { useDragDrop } from '../composables/useDragDrop';
import { usePasswordVisibility } from '../composables/usePasswordVisibility';
import ProgressBar from './ProgressBar.vue';

// Initialize composables
const fileOps = useFileOps();
const tauri = useTauri();

// Password visibility toggle
const { isPasswordVisible, togglePasswordVisibility } = usePasswordVisibility();

// Progress tracking for decryption operation
const { progress, isActive: showProgress, startListening, stopListening } = useProgress();

// Drag-and-drop file handling
const { isDragging, handleDragOver, handleDragLeave, handleDrop, setupDragDrop } = useDragDrop(
  (path) => fileOps.setInputPath(path, false) // false = decryption mode
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
        />
        <button
          @click="handleSelectFile"
          class="btn btn-primary"
          :disabled="fileOps.isProcessing.value"
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
        />
        <button
          @click="handleSelectOutput"
          class="btn btn-secondary"
          :disabled="fileOps.isProcessing.value"
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
        />
        <button
          type="button"
          class="password-toggle-btn"
          @click="togglePasswordVisibility"
          :disabled="fileOps.isProcessing.value"
          :aria-label="isPasswordVisible ? 'Hide password' : 'Show password'"
        >
          <!-- Eye icon (show) -->
          <svg v-if="!isPasswordVisible" xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path>
            <circle cx="12" cy="12" r="3"></circle>
          </svg>
          <!-- Eye-off icon (hide) -->
          <svg v-else xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"></path>
            <line x1="1" y1="1" x2="23" y2="23"></line>
          </svg>
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
