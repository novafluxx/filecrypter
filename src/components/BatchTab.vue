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
import PasswordStrengthMeter from './PasswordStrengthMeter.vue';
import type { BatchProgress, BatchResult, FileResult } from '../types/crypto';

// Initialize Tauri composable
const tauri = useTauri();

// State
const mode = ref<'encrypt' | 'decrypt'>('encrypt');
const inputPaths = ref<string[]>([]);
const outputDir = ref('');
const password = ref('');
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

    if (mode.value === 'encrypt') {
      result = await tauri.batchEncrypt(inputPaths.value, outputDir.value, password.value);
    } else {
      result = await tauri.batchDecrypt(inputPaths.value, outputDir.value, password.value);
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
    statusMessage.value = `Batch operation failed: ${error}`;
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
          <span class="file-name" :title="path">{{ getFileName(path) }}</span>
          <span
            v-if="batchResult?.files[index]?.error"
            class="file-error-msg"
            :title="batchResult.files[index].error ?? ''"
          >
            {{ batchResult.files[index].error }}
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

    <!-- Password Input -->
    <div class="form-group">
      <label>Password:</label>
      <input
        type="password"
        v-model="password"
        :placeholder="mode === 'encrypt' ? 'Enter a strong password (min 8 characters)' : 'Enter decryption password'"
        :autocomplete="mode === 'encrypt' ? 'new-password' : 'current-password'"
        class="password-input"
        :disabled="isProcessing"
      />
      <!-- Password strength meter (encryption only) -->
      <PasswordStrengthMeter
        v-if="mode === 'encrypt' && password.length > 0"
        :strength="passwordStrength"
        :show-feedback="!isPasswordValid"
      />
      <!-- Info hint for decryption -->
      <div v-if="mode === 'decrypt' && password.length === 0" class="hint hint-info">
        Enter the password used to encrypt these files
      </div>
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
  padding: 24px 0;
  position: relative;
  min-height: 300px;
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
}

.password-input:disabled,
.file-input:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* File List */
.file-list {
  margin-top: 12px;
  max-height: 200px;
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

/* Hints */
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
