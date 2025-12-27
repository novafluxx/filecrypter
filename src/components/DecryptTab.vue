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
import ProgressBar from './ProgressBar.vue';

// Initialize composables
const fileOps = useFileOps();
const tauri = useTauri();

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
  <div
    class="tab-content"
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
          placeholder="Auto-suggested..."
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
      <div class="hint hint-info">
        If the output name already exists, we'll save as "name (1)".
      </div>
    </div>

    <!-- Password Input Section -->
    <div class="form-group">
      <label for="decrypt-password">Password:</label>
      <input
        id="decrypt-password"
        type="password"
        :value="fileOps.password.value"
        @input="fileOps.setPassword(($event.target as HTMLInputElement).value)"
        placeholder="Enter decryption password"
        autocomplete="current-password"
        class="password-input"
        :disabled="fileOps.isProcessing.value"
      />
      <!-- Info hint -->
      <div v-if="fileOps.password.value.length === 0" class="hint hint-info">
        Enter the password used to encrypt this file
      </div>
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
</template>

<style scoped>
/* Shared styles with EncryptTab - could be extracted to global styles */

.tab-content {
  padding: 24px 0;
  position: relative;
  min-height: 300px;
}

/* Drag-and-drop styles */
.drop-zone-active {
  outline: 2px dashed #4a90e2;
  outline-offset: -2px;
  background-color: rgba(74, 144, 226, 0.05);
  border-radius: 8px;
}

.drop-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(74, 144, 226, 0.1);
  border-radius: 8px;
  font-size: 18px;
  font-weight: 500;
  color: #4a90e2;
  z-index: 10;
  pointer-events: none;
}

.form-group {
  margin-bottom: 20px;
}

label {
  display: block;
  margin-bottom: 8px;
  font-weight: 500;
  color: var(--text-primary);
  font-size: 14px;
}

.file-input-group {
  display: flex;
  gap: 8px;
}

.file-input,
.password-input {
  flex: 1;
  padding: 10px 12px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  font-size: 14px;
  font-family: inherit;
  transition: border-color 0.2s, background-color 0.2s;
  background-color: var(--bg-secondary);
  color: var(--text-primary);
}

.file-input:focus,
.password-input:focus {
  outline: none;
  border-color: var(--accent-primary);
}

.file-input {
  background-color: var(--input-bg);
  cursor: default;
}

.password-input:disabled,
.file-input:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn {
  padding: 10px 20px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  font-family: inherit;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-primary {
  background-color: var(--accent-primary);
  color: white;
}

.btn-primary:hover:not(:disabled) {
  background-color: var(--accent-hover);
}

.btn-secondary {
  background-color: var(--btn-secondary-bg);
  color: white;
}

.btn-secondary:hover:not(:disabled) {
  background-color: var(--btn-secondary-hover);
}

.btn-action {
  width: 100%;
  padding: 14px;
  background-color: var(--accent-secondary);
  color: white;
  font-size: 16px;
  margin-top: 8px;
}

.btn-action:hover:not(:disabled) {
  filter: brightness(0.9);
}

.hint {
  margin-top: 6px;
  font-size: 12px;
  padding: 6px 10px;
  border-radius: 4px;
}

.hint-info {
  background-color: var(--info-bg);
  color: var(--info-text);
  border: 1px solid var(--info-border);
}

.checkbox-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 500;
  font-size: 13px;
  color: var(--text-primary);
}

.checkbox-row input {
  accent-color: var(--accent-primary);
}

.status-message {
  margin-top: 16px;
  padding: 12px;
  border-radius: 6px;
  font-size: 14px;
}

.status-success {
  background-color: var(--success-bg);
  color: var(--success-text);
  border: 1px solid var(--success-border);
}

.status-error {
  background-color: var(--error-bg);
  color: var(--error-text);
  border: 1px solid var(--error-border);
}

.status-info {
  background-color: var(--info-bg);
  color: var(--info-text);
  border: 1px solid var(--info-border);
}
</style>
