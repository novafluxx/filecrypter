<!-- components/BatchTab.vue - Batch Encryption/Decryption Interface -->
<!--
  This component provides the UI for batch processing multiple files.

  Features:
  - Multiple file selection
  - Mode toggle (encrypt/decrypt)
  - Output directory selection
  - Password input with strength meter (encrypt mode)
  - Real-time batch progress
  - Results display with per-file status
-->

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useTauri } from '../composables/useTauri';
import { usePasswordStrength } from '../composables/usePasswordStrength';
import { usePasswordVisibility } from '../composables/usePasswordVisibility';
import PasswordStrengthMeter from './PasswordStrengthMeter.vue';
import type { BatchProgress, BatchResult, FileResult } from '../types/crypto';

// Initialize Tauri composable
const tauri = useTauri();

// Password visibility toggle
const { isPasswordVisible, togglePasswordVisibility } = usePasswordVisibility();

// State
const mode = ref<'encrypt' | 'decrypt'>('encrypt');
const inputPaths = ref<string[]>([]);
const outputDir = ref('');
const password = ref('');
const neverOverwrite = ref(true);
const isProcessing = ref(false);
const statusMessage = ref('');
const statusType = ref<'success' | 'error' | 'info'>('info');
const batchResult = ref<BatchResult | null>(null);

// Batch progress
const batchProgress = ref<BatchProgress | null>(null);
const showProgress = ref(false);

// Password strength (only relevant for encryption)
const { strength: passwordStrength } = usePasswordStrength(password);

// Event listener cleanup
let unlistenProgress: UnlistenFn | null = null;

// Computed properties
const isPasswordValid = computed(() => password.value.length >= 8);

const isFormValid = computed(() =>
  inputPaths.value.length > 0 &&
  outputDir.value.length > 0 &&
  (mode.value === 'decrypt' || isPasswordValid.value) &&
  password.value.length > 0 &&
  !isProcessing.value
);

const fileCount = computed(() => inputPaths.value.length);

// Extract filename from full path
function getFileName(path: string): string {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

/**
 * Safe error messages to display to users.
 * Prevents information leakage by mapping backend errors to safe messages.
 */
const SAFE_ERROR_MESSAGES: Record<string, string> = {
  InvalidPassword: 'Incorrect password or corrupted file',
  FileNotFound: 'File could not be accessed',
  FileTooLarge: 'File is too large for this operation',
  TooManyFiles: 'Too many files selected for batch operation',
  InvalidPath: 'Invalid file path',
  permission: 'Permission denied',
  default: 'Operation failed - please try again',
};

/**
 * Sanitize error messages for user display.
 */
function sanitizeErrorMessage(error: unknown): string {
  const errStr = String(error);
  for (const [key, safeMsg] of Object.entries(SAFE_ERROR_MESSAGES)) {
    if (key !== 'default' && errStr.includes(key)) {
      return safeMsg;
    }
  }
  return SAFE_ERROR_MESSAGES['default'] ?? 'Operation failed';
}

// Setup progress listener
async function startProgressListener() {
  unlistenProgress = await listen<BatchProgress>('batch-progress', (event) => {
    batchProgress.value = event.payload;
    showProgress.value = event.payload.stage !== 'complete';
  });
}

// Stop progress listener
function stopProgressListener() {
  if (unlistenProgress) {
    unlistenProgress();
    unlistenProgress = null;
  }
}

// Cleanup on unmount
onUnmounted(() => {
  stopProgressListener();
});

// Handle file selection
async function handleSelectFiles() {
  const filters = mode.value === 'decrypt'
    ? [
        { name: 'Encrypted Files', extensions: ['encrypted'] },
        { name: 'All Files', extensions: ['*'] }
      ]
    : undefined;

  const paths = await tauri.selectMultipleFiles(
    mode.value === 'encrypt' ? 'Select Files to Encrypt' : 'Select Files to Decrypt',
    filters
  );

  if (paths.length > 0) {
    inputPaths.value = paths;
    batchResult.value = null;
    statusMessage.value = '';
  }
}

// Handle output directory selection
async function handleSelectOutputDir() {
  const dir = await tauri.selectDirectory('Select Output Directory');
  if (dir) {
    outputDir.value = dir;
  }
}

// Remove a file from the list
function removeFile(index: number) {
  inputPaths.value = inputPaths.value.filter((_, i) => i !== index);
  batchResult.value = null;
}

// Clear all files
function clearFiles() {
  inputPaths.value = [];
  batchResult.value = null;
  statusMessage.value = '';
}

// Handle batch operation
async function handleBatchOperation() {
  if (!isFormValid.value) return;

  isProcessing.value = true;
  batchResult.value = null;
  statusMessage.value = '';
  showProgress.value = true;

  await startProgressListener();

  try {
    let result: BatchResult;

    const allowOverwrite = !neverOverwrite.value;

    if (mode.value === 'encrypt') {
      result = await tauri.batchEncrypt(
        inputPaths.value,
        outputDir.value,
        password.value,
        allowOverwrite
      );
    } else {
      result = await tauri.batchDecrypt(
        inputPaths.value,
        outputDir.value,
        password.value,
        allowOverwrite
      );
    }

    batchResult.value = result;

    // Clear password for security
    password.value = '';

    // Set status message
    if (result.failed_count === 0) {
      statusMessage.value = `Successfully ${mode.value === 'encrypt' ? 'encrypted' : 'decrypted'} ${result.success_count} file${result.success_count !== 1 ? 's' : ''}`;
      statusType.value = 'success';
    } else if (result.success_count === 0) {
      statusMessage.value = `Failed to ${mode.value} all ${result.failed_count} files`;
      statusType.value = 'error';
    } else {
      statusMessage.value = `Completed: ${result.success_count} succeeded, ${result.failed_count} failed`;
      statusType.value = 'info';
    }
  } catch (error) {
    statusMessage.value = sanitizeErrorMessage(error);
    statusType.value = 'error';
  } finally {
    isProcessing.value = false;
    showProgress.value = false;
    stopProgressListener();
  }
}

// Switch mode and clear state
function switchMode(newMode: 'encrypt' | 'decrypt') {
  if (mode.value !== newMode) {
    mode.value = newMode;
    inputPaths.value = [];
    batchResult.value = null;
    statusMessage.value = '';
    password.value = '';
  }
}
</script>

<template>
  <div class="tab-content">
    <!-- Mode Toggle -->
    <div class="mode-toggle">
      <button
        class="mode-btn"
        :class="{ active: mode === 'encrypt' }"
        @click="switchMode('encrypt')"
        :disabled="isProcessing"
      >
        Encrypt
      </button>
      <button
        class="mode-btn"
        :class="{ active: mode === 'decrypt' }"
        @click="switchMode('decrypt')"
        :disabled="isProcessing"
      >
        Decrypt
      </button>
    </div>

    <!-- File Selection -->
    <div class="form-group">
      <label>{{ mode === 'encrypt' ? 'Files to Encrypt' : 'Files to Decrypt' }}:</label>
      <div class="file-input-group">
        <div class="file-count-display">
          {{ fileCount }} file{{ fileCount !== 1 ? 's' : '' }} selected
        </div>
        <button
          @click="handleSelectFiles"
          class="btn btn-primary"
          :disabled="isProcessing"
        >
          Browse
        </button>
      </div>

      <!-- Selected Files List -->
      <div v-if="inputPaths.length > 0" class="file-list">
        <div
          v-for="(path, index) in inputPaths"
          :key="path"
          class="file-item"
          :class="{
            'file-success': batchResult?.files[index]?.success,
            'file-error': batchResult?.files[index]?.success === false
          }"
        >
          <span class="file-name" :title="getFileName(path)">{{ getFileName(path) }}</span>
          <span
            v-if="batchResult?.files[index]?.error"
            class="file-error-msg"
            :title="sanitizeErrorMessage(batchResult.files[index].error)"
          >
            {{ sanitizeErrorMessage(batchResult.files[index].error) }}
          </span>
          <button
            v-if="!isProcessing && !batchResult"
            @click="removeFile(index)"
            class="remove-btn"
            title="Remove file"
          >
            &times;
          </button>
        </div>
        <button
          v-if="inputPaths.length > 1 && !isProcessing && !batchResult"
          @click="clearFiles"
          class="btn-link"
        >
          Clear all
        </button>
      </div>
    </div>

    <!-- Output Directory -->
    <div class="form-group">
      <label>Output Directory:</label>
      <div class="file-input-group">
        <input
          type="text"
          :value="outputDir"
          readonly
          placeholder="Select output directory..."
          class="file-input"
        />
        <button
          @click="handleSelectOutputDir"
          class="btn btn-secondary"
          :disabled="isProcessing"
        >
          Browse
        </button>
      </div>
    </div>

    <!-- Output Safety Options -->
    <div class="form-group">
      <label class="checkbox-row">
        <input
          type="checkbox"
          v-model="neverOverwrite"
          :disabled="isProcessing"
        />
        Never overwrite existing files (auto-rename on conflicts)
      </label>
      <p class="hint-text">
        If a filename already exists, we'll save as "name (1)".
      </p>
    </div>

    <!-- Password Input -->
    <div class="form-group password-section">
      <label>Password:</label>
      <div class="password-input-wrapper">
        <input
          :type="isPasswordVisible ? 'text' : 'password'"
          v-model="password"
          :placeholder="mode === 'encrypt' ? 'Enter password (min 8 characters)' : 'Enter decryption password'"
          :autocomplete="mode === 'encrypt' ? 'new-password' : 'current-password'"
          class="password-input"
          :disabled="isProcessing"
        />
        <button
          type="button"
          class="password-toggle-btn"
          @click="togglePasswordVisibility"
          :disabled="isProcessing"
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
      <!-- Password strength meter (encryption only) -->
      <PasswordStrengthMeter
        v-if="mode === 'encrypt' && password.length > 0"
        :strength="passwordStrength"
        :show-feedback="!isPasswordValid"
      />
      <!-- Info hint for decryption -->
      <p v-if="mode === 'decrypt' && password.length === 0" class="hint-text">
        Enter the password used to encrypt these files
      </p>
    </div>

    <!-- Action Button -->
    <button
      @click="handleBatchOperation"
      class="btn btn-action"
      :disabled="!isFormValid"
    >
      <span v-if="isProcessing">
        {{ mode === 'encrypt' ? 'Encrypting' : 'Decrypting' }}...
      </span>
      <span v-else>
        {{ mode === 'encrypt' ? 'Encrypt' : 'Decrypt' }} {{ fileCount }} File{{ fileCount !== 1 ? 's' : '' }}
      </span>
    </button>

    <!-- Progress Bar -->
    <div v-if="showProgress && batchProgress" class="progress-container">
      <div class="progress-bar-bg">
        <div
          class="progress-bar-fill"
          :style="{ width: `${batchProgress.percent}%` }"
        ></div>
      </div>
      <div class="progress-info">
        <span class="progress-message">
          {{ batchProgress.stage === 'complete' ? 'Complete' : `Processing: ${batchProgress.current_file}` }}
        </span>
        <span class="progress-percent">
          {{ batchProgress.file_index + 1 }}/{{ batchProgress.total_files }}
        </span>
      </div>
    </div>

    <!-- Status Message -->
    <div
      v-if="statusMessage"
      class="status-message"
      :class="`status-${statusType}`"
    >
      {{ statusMessage }}
    </div>
  </div>
</template>

<style scoped>
.tab-content {
  padding: 8px 0;
  position: relative;
  min-height: 200px;
}

/* Mode Toggle */
.mode-toggle {
  display: flex;
  gap: 8px;
  margin-bottom: 20px;
  padding: 4px;
  background: var(--bg-tertiary);
  border-radius: 8px;
}

.mode-btn {
  flex: 1;
  padding: 10px 16px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  font-family: inherit;
}

.mode-btn:hover:not(:disabled) {
  color: var(--text-primary);
}

.mode-btn.active {
  background: var(--accent-primary);
  color: white;
}

.mode-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* Form Groups */
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

.file-count-display {
  flex: 1;
  padding: 10px 12px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  font-size: 14px;
  background-color: var(--input-bg);
  color: var(--text-secondary);
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

/* File List */
.file-list {
  margin-top: 12px;
  max-height: 150px;
  overflow-y: auto;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--bg-tertiary);
}

.file-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border-color);
  font-size: 13px;
}

.file-item:last-child {
  border-bottom: none;
}

.file-item.file-success {
  background-color: var(--success-bg);
}

.file-item.file-error {
  background-color: var(--error-bg);
}

.file-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-primary);
}

.file-error-msg {
  font-size: 11px;
  color: var(--error-text);
  max-width: 150px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.remove-btn {
  width: 20px;
  height: 20px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--text-muted);
  font-size: 16px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.remove-btn:hover {
  background: var(--error-bg);
  color: var(--error-text);
}

.btn-link {
  display: block;
  width: 100%;
  padding: 8px;
  border: none;
  background: transparent;
  color: var(--accent-primary);
  font-size: 12px;
  cursor: pointer;
  text-align: center;
}

.btn-link:hover {
  text-decoration: underline;
}

/* Buttons */
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

/* Progress */
.progress-container {
  margin: 16px 0;
  padding: 12px;
  background: var(--bg-tertiary);
  border-radius: 8px;
  border: 1px solid var(--border-color);
}

.progress-bar-bg {
  height: 8px;
  background: var(--border-color);
  border-radius: 4px;
  overflow: hidden;
}

.progress-bar-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent-primary), var(--accent-secondary));
  border-radius: 4px;
  transition: width 0.3s ease;
}

.progress-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 8px;
}

.progress-message {
  font-size: 13px;
  color: var(--text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 70%;
}

.progress-percent {
  font-size: 13px;
  font-weight: 600;
  color: var(--accent-primary);
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

/* Hints */
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

/* Status Message */
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
