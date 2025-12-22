<!-- components/EncryptTab.vue - File Encryption Interface -->
<!--
  This component provides the UI for encrypting files.

  Features:
  - File selection via native dialogs
  - Auto-suggested output filename
  - Password input with validation
  - Real-time form validation
  - Status messages (success/error/info)

  Vue Composition API:
  - <script setup> provides cleaner syntax
  - Reactive state managed by composables
  - Computed properties for validation
-->

<script setup lang="ts">
import { onMounted } from 'vue';
import { useFileOps } from '../composables/useFileOps';
import { useTauri } from '../composables/useTauri';
import { usePasswordStrength } from '../composables/usePasswordStrength';
import { useProgress } from '../composables/useProgress';
import { useDragDrop } from '../composables/useDragDrop';
import PasswordStrengthMeter from './PasswordStrengthMeter.vue';
import ProgressBar from './ProgressBar.vue';

// Initialize composables
// These provide reactive state and methods for file operations
const fileOps = useFileOps();
const tauri = useTauri();

// Password strength analysis
// Provides reactive feedback as user types their password
const { strength: passwordStrength } = usePasswordStrength(fileOps.password);

// Progress tracking for encryption operation
const { progress, isActive: showProgress, startListening, stopListening } = useProgress();

// Drag-and-drop file handling
const { isDragging, handleDragOver, handleDragLeave, handleDrop, setupDragDrop } = useDragDrop(
  (path) => fileOps.setInputPath(path, true) // true = encryption mode
);

// Setup drag-drop on mount
onMounted(() => {
  setupDragDrop();
});

/**
 * Handle file selection for encryption
 *
 * Opens native file picker, then auto-suggests output filename
 */
async function handleSelectFile() {
  const path = await tauri.selectFile('Select File to Encrypt');

  if (path) {
    // setInputPath automatically suggests output path with .encrypted extension
    fileOps.setInputPath(path, true); // true = encryption mode
  }
}

/**
 * Handle output path selection
 *
 * Allows user to change the auto-suggested output path
 */
async function handleSelectOutput() {
  const path = await tauri.selectSavePath(
    'Save Encrypted File As',
    fileOps.outputPath.value,
    [{ name: 'Encrypted Files', extensions: ['encrypted'] }]
  );

  if (path) {
    fileOps.setOutputPath(path);
  }
}

/**
 * Handle encryption button click
 *
 * Validates form, starts progress listener, calls Rust backend, shows result
 */
async function handleEncrypt() {
  // Start listening for progress events before encryption begins
  await startListening();

  try {
    const success = await fileOps.performEncrypt();
    // If successful, optionally reset some fields
    // For now, we keep the paths so user can encrypt again with different password
  } finally {
    // Progress listener auto-stops on completion, but stop on error too
    if (!showProgress.value) {
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
      Drop file here to encrypt
    </div>

    <!-- File Input Section -->
    <div class="form-group">
      <label for="encrypt-input">File to Encrypt:</label>
      <div class="file-input-group">
        <input
          id="encrypt-input"
          type="text"
          :value="fileOps.inputPath.value"
          readonly
          placeholder="Select or drag a file..."
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
      <label for="encrypt-output">Save Encrypted File As:</label>
      <div class="file-input-group">
        <input
          id="encrypt-output"
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

    <!-- Password Input Section -->
    <div class="form-group">
      <label for="encrypt-password">Password:</label>
      <input
        id="encrypt-password"
        type="password"
        :value="fileOps.password.value"
        @input="fileOps.setPassword(($event.target as HTMLInputElement).value)"
        placeholder="Enter a strong password (min 8 characters)"
        autocomplete="new-password"
        class="password-input"
        :disabled="fileOps.isProcessing.value"
      />
      <!-- Password strength meter -->
      <PasswordStrengthMeter
        v-if="fileOps.password.value.length > 0"
        :strength="passwordStrength"
        :show-feedback="!fileOps.isPasswordValid.value"
      />
    </div>

    <!-- Encrypt Button -->
    <button
      @click="handleEncrypt"
      class="btn btn-action"
      :disabled="!fileOps.isFormValid.value"
    >
      <span v-if="fileOps.isProcessing.value">Encrypting...</span>
      <span v-else>Encrypt File</span>
    </button>

    <!-- Progress Bar (shown during encryption) -->
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
/* Component-specific styles */
/* These styles are scoped to this component only */

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

.hint-warning {
  background-color: var(--warning-bg);
  color: var(--warning-text);
  border: 1px solid var(--warning-border);
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
