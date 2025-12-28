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
import { usePasswordVisibility } from '../composables/usePasswordVisibility';
import PasswordStrengthMeter from './PasswordStrengthMeter.vue';
import ProgressBar from './ProgressBar.vue';

// Initialize composables
// These provide reactive state and methods for file operations
const fileOps = useFileOps();
const tauri = useTauri();

// Password visibility toggle
const { isPasswordVisible, togglePasswordVisibility } = usePasswordVisibility();

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

  let success = false;
  try {
    success = await fileOps.performEncrypt();
    // If successful, optionally reset some fields
    // For now, we keep the paths so user can encrypt again with different password
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
          placeholder="Will auto-generate from input filename..."
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
      <label for="encrypt-password">Password:</label>
      <div class="password-input-wrapper">
        <input
          id="encrypt-password"
          :type="isPasswordVisible ? 'text' : 'password'"
          :value="fileOps.password.value"
          @input="fileOps.setPassword(($event.target as HTMLInputElement).value)"
          placeholder="Enter password (min 8 characters)"
          autocomplete="new-password"
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
  padding: 8px 0;
  position: relative;
  min-height: 200px;
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
  margin-bottom: 12px;
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
  border: 2px dashed var(--border-color);
  min-height: 44px;
  padding: 12px;
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
  font-weight: 600;
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

.hint-text {
  margin-top: 4px;
  font-size: 12px;
  color: var(--text-muted);
  font-style: italic;
}

.hint-warning {
  background-color: var(--warning-bg);
  color: var(--warning-text);
  border: 1px solid var(--warning-border);
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

/* Password visibility toggle */
.password-input-wrapper {
  position: relative;
  display: flex;
  align-items: center;
}

.password-input-wrapper .password-input {
  padding-right: 44px;
}

.password-toggle-btn {
  position: absolute;
  right: 8px;
  top: 50%;
  transform: translateY(-50%);
  background: transparent;
  border: none;
  padding: 6px;
  cursor: pointer;
  color: var(--text-muted);
  transition: color 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
}

.password-toggle-btn:hover:not(:disabled) {
  color: var(--accent-primary);
  background: var(--bg-tertiary);
}

.password-toggle-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Password section spacing */
.password-section {
  margin-top: 16px;
  padding-top: 12px;
  border-top: 1px solid var(--border-color);
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
