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
import { ref, computed, onUnmounted } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useTauri } from '../composables/useTauri';
import { usePasswordStrength } from '../composables/usePasswordStrength';
import { usePasswordVisibility } from '../composables/usePasswordVisibility';
import { sanitizeErrorMessage } from '../utils/errorSanitizer';
import PasswordStrengthMeter from './PasswordStrengthMeter.vue';
import IconEye from './icons/IconEye.vue';
import IconEyeOff from './icons/IconEyeOff.vue';
import type { BatchProgress, BatchResult, FileResult } from '../types/crypto';
import { MIN_PASSWORD_LENGTH } from '../constants';

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
const isPasswordValid = computed(() => password.value.length >= MIN_PASSWORD_LENGTH);

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
  return path.split(/[/\\]/).at(-1) ?? path;
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
    <div class="content-panel">
      <!-- Mode Toggle -->
      <div class="mode-toggle">
      <button
        class="mode-btn"
        :class="{ active: mode === 'encrypt' }"
        @click="switchMode('encrypt')"
        :disabled="isProcessing"
        title="Batch encrypt multiple files"
      >
        Encrypt
      </button>
      <button
        class="mode-btn"
        :class="{ active: mode === 'decrypt' }"
        @click="switchMode('decrypt')"
        :disabled="isProcessing"
        title="Batch decrypt multiple files"
      >
        Decrypt
      </button>
    </div>

    <!-- Compression Info Banner (encryption mode only) -->
    <div v-if="mode === 'encrypt'" class="info-banner">
      Compression is automatically enabled for batch operations.
      Files are compressed with ZSTD before encryption for optimal size reduction.
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
          title="Choose multiple files to process"
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
          title="Remove all selected files"
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
          title="Auto-filled output folder; click Browse to pick a different one"
        />
        <button
          @click="handleSelectOutputDir"
          class="btn btn-secondary"
          :disabled="isProcessing"
          title="Choose the output folder"
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
          title="Prevent overwriting by auto-renaming on name conflicts"
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
          :title="mode === 'encrypt' ? 'Enter a strong password (at least 8 characters)' : 'Enter the password used to encrypt these files'"
        />
        <button
          type="button"
          class="password-toggle-btn"
          @click="togglePasswordVisibility"
          :disabled="isProcessing"
          :aria-label="isPasswordVisible ? 'Hide password' : 'Show password'"
          :title="isPasswordVisible ? 'Hide password' : 'Show password'"
        >
          <IconEye v-if="!isPasswordVisible" />
          <IconEyeOff v-else />
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
      :title="mode === 'encrypt' ? 'Encrypt all selected files' : 'Decrypt all selected files'"
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
        role="status"
        aria-live="polite"
      >
        {{ statusMessage }}
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

/* Mode Toggle (unique to BatchTab) */
.mode-toggle {
  display: flex;
  gap: 4px;
  margin-bottom: 16px;
  padding: 4px;
  background: var(--panel-alt);
  border-radius: 4px;
  border: 1px solid var(--border);
}

.mode-btn {
  flex: 1;
  padding: 6px 12px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--muted);
  font-size: 17px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
  font-family: inherit;
}

.mode-btn:hover:not(:disabled):not(.active) {
  color: var(--text);
  background: var(--border);
}

.mode-btn.active {
  background: var(--accent);
  color: white;
}

.mode-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* File Count Display */
.file-count-display {
  flex: 1;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 4px;
  font-size: 19px;
  background-color: var(--field);
  color: var(--muted);
}

/* File List */
.file-list {
  margin-top: 12px;
  max-height: 180px;
  overflow-y: auto;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: var(--panel-alt);
}

.file-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
  font-size: 16px;
  transition: background-color 0.15s;
}

.file-item:last-child {
  border-bottom: none;
}

.file-item:hover {
  background-color: var(--panel);
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
  color: var(--text);
}

.file-error-msg {
  font-size: 11px;
  color: var(--error-text);
  max-width: 180px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.remove-btn {
  width: 24px;
  height: 24px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--muted);
  font-size: 18px;
  line-height: 1;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
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
  color: var(--accent);
  font-size: 16px;
  font-weight: 500;
  cursor: pointer;
  text-align: center;
  transition: all 0.15s;
}

.btn-link:hover {
  text-decoration: underline;
  background: var(--panel);
}

/* Progress Container (unique to BatchTab) */
.progress-container {
  margin: 16px 0;
  padding: 12px;
  background: var(--panel-alt);
  border-radius: 6px;
  border: 1px solid var(--border);
}

.progress-bar-bg {
  height: 6px;
  background: var(--border);
  border-radius: 3px;
  overflow: hidden;
}

.progress-bar-fill {
  height: 100%;
  background: var(--accent);
  border-radius: 3px;
  transition: width 0.3s ease;
}

.progress-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 8px;
}

.progress-message {
  font-size: 16px;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 70%;
}

.progress-percent {
  font-size: 16px;
  font-weight: 600;
  color: var(--accent);
}

/* Info Banner */
.info-banner {
  padding: 12px 16px;
  margin-bottom: 16px;
  background: var(--panel-alt);
  border: 1px solid var(--border);
  border-left: 3px solid var(--accent);
  border-radius: 4px;
  font-size: 14px;
  color: var(--muted);
  line-height: 1.5;
}
</style>
