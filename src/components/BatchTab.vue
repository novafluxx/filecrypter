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
import { NButton, NButtonGroup, NCheckbox, NAlert, NInput, NRadioGroup, NRadio, useDialog } from 'naive-ui';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useTauri } from '../composables/useTauri';
import { usePasswordStrength } from '../composables/usePasswordStrength';
import { useDragDrop } from '../composables/useDragDrop';
import { useSettings } from '../composables/useSettings';
import { useSettingsSync } from '../composables/useSettingsSync';
import { sanitizeErrorMessage } from '../utils/errorSanitizer';
import KeyFileSection from './KeyFileSection.vue';
import PasswordSection from './PasswordSection.vue';
import StatusMessage from './StatusMessage.vue';
import type { BatchProgress, BatchResult, ArchiveProgress, ArchiveResult, BatchMode } from '../types/crypto';
import { MIN_PASSWORD_LENGTH } from '../constants';

// Initialize composables
const dialog = useDialog();
const tauri = useTauri();
const settings = useSettings();

// State
const mode = ref<'encrypt' | 'decrypt'>('encrypt');
const batchMode = ref<BatchMode>('individual');
const inputPaths = ref<string[]>([]);
const outputDir = ref('');
const password = ref('');
const neverOverwrite = ref(true);
const keyFilePath = ref('');
const archiveName = ref('');

// Sync settings to local state (initial + reactive updates)
useSettingsSync(settings, {
  neverOverwrite: neverOverwrite,
  outputDirectory: outputDir,
});
const isProcessing = ref(false);
const statusMessage = ref('');
const statusType = ref<'success' | 'error' | 'info'>('info');
const batchResult = ref<BatchResult | null>(null);

// Batch progress
const batchProgress = ref<BatchProgress | null>(null);
const archiveProgress = ref<ArchiveProgress | null>(null);
const showProgress = ref(false);

// Password strength (only relevant for encryption)
const { strength: passwordStrength } = usePasswordStrength(password);

// Drop zone element reference for drag-and-drop
const dropZoneRef = ref<HTMLElement>();

// Drag-and-drop file handling
const { isDragging, handleDragOver, handleDragLeave, handleDrop, setupDragDrop } = useDragDrop(
  (paths) => {
    // Add all dropped files to the input paths (avoid duplicates)
    const newPaths = paths.filter(path => !inputPaths.value.includes(path));
    if (newPaths.length > 0) {
      inputPaths.value = [...inputPaths.value, ...newPaths];
      batchResult.value = null;
      statusMessage.value = '';
    }
  },
  dropZoneRef
);

// Setup drag-drop on mount
onMounted(() => {
  setupDragDrop();
});

// Event listener cleanup
let unlistenProgress: UnlistenFn | null = null;
let unlistenArchiveProgress: UnlistenFn | null = null;

// Computed properties
const isPasswordValid = computed(() => password.value.length >= MIN_PASSWORD_LENGTH);

// Invalid filename characters (cross-platform: Windows + macOS/Linux)
const INVALID_FILENAME_CHARS = /[/\\:*?"<>|]/;

const archiveNameError = computed(() => {
  if (!archiveName.value) return null; // Empty is allowed (auto-generated)
  if (INVALID_FILENAME_CHARS.test(archiveName.value)) {
    return 'Archive name contains invalid characters: / \\ : * ? " < > |';
  }
  if (archiveName.value.startsWith('.')) {
    return 'Archive name should not start with a dot';
  }
  return null;
});

const isArchiveNameValid = computed(() => !archiveNameError.value);

const isFormValid = computed(() =>
  inputPaths.value.length > 0 &&
  outputDir.value.length > 0 &&
  (mode.value === 'decrypt' || isPasswordValid.value) &&
  password.value.length > 0 &&
  !isProcessing.value &&
  (batchMode.value !== 'archive' || mode.value !== 'encrypt' || isArchiveNameValid.value)
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

// Setup archive progress listener
async function startArchiveProgressListener() {
  unlistenArchiveProgress = await listen<ArchiveProgress>('archive-progress', (event) => {
    archiveProgress.value = event.payload;
    showProgress.value = event.payload.phase !== 'complete';
  });
}

// Stop progress listener
function stopProgressListener() {
  if (unlistenProgress) {
    unlistenProgress();
    unlistenProgress = null;
  }
  if (unlistenArchiveProgress) {
    unlistenArchiveProgress();
    unlistenArchiveProgress = null;
  }
}

// Cleanup on unmount
onUnmounted(() => {
  stopProgressListener();
});

// Handle file selection
async function handleSelectFiles() {
  // For archive decrypt mode, select a single archive file
  if (batchMode.value === 'archive' && mode.value === 'decrypt') {
    const path = await tauri.selectFile(
      'Select Encrypted Archive',
      [
        { name: 'Encrypted Archives', extensions: ['encrypted'] },
        { name: 'All Files', extensions: ['*'] }
      ]
    );

    if (path) {
      inputPaths.value = [path];
      batchResult.value = null;
      statusMessage.value = '';
    }
    return;
  }

  // For other modes, select multiple files
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

// Show confirmation dialog before batch operation
function confirmBatchOperation() {
  if (!isFormValid.value) return;

  const action = mode.value === 'encrypt' ? 'encrypt' : 'decrypt';
  const count = inputPaths.value.length;
  const content = batchMode.value === 'archive'
    ? `Are you sure you want to ${action} ${count} file${count !== 1 ? 's' : ''} as an archive?`
    : `Are you sure you want to ${action} ${count} file${count !== 1 ? 's' : ''} individually?`;

  dialog.warning({
    title: 'Confirm Batch Operation',
    content,
    positiveText: 'Confirm',
    negativeText: 'Cancel',
    onPositiveClick: () => {
      handleBatchOperation();
    },
  });
}

// Handle batch operation
async function handleBatchOperation() {
  if (!isFormValid.value) return;

  isProcessing.value = true;
  batchResult.value = null;
  statusMessage.value = '';
  showProgress.value = true;
  batchProgress.value = null;
  archiveProgress.value = null;

  const allowOverwrite = !neverOverwrite.value;

  // Choose between individual and archive mode
  if (batchMode.value === 'archive') {
    await handleArchiveOperation(allowOverwrite);
  } else {
    await handleIndividualOperation(allowOverwrite);
  }
}

// Handle individual file batch operation
async function handleIndividualOperation(allowOverwrite: boolean) {
  await startProgressListener();

  try {
    let result: BatchResult;

    if (mode.value === 'encrypt') {
      result = await tauri.batchEncrypt(
        inputPaths.value,
        outputDir.value,
        password.value,
        allowOverwrite,
        keyFilePath.value || undefined
      );
    } else {
      result = await tauri.batchDecrypt(
        inputPaths.value,
        outputDir.value,
        password.value,
        allowOverwrite,
        keyFilePath.value || undefined
      );
    }

    batchResult.value = result;

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
    // Always clear password, even when the operation fails
    password.value = '';
    isProcessing.value = false;
    showProgress.value = false;
    stopProgressListener();
  }
}

// Handle archive mode operation
async function handleArchiveOperation(allowOverwrite: boolean) {
  await startArchiveProgressListener();

  try {
    let result: ArchiveResult;

    if (mode.value === 'encrypt') {
      result = await tauri.batchEncryptArchive(
        inputPaths.value,
        outputDir.value,
        password.value,
        archiveName.value || undefined,
        allowOverwrite,
        keyFilePath.value || undefined
      );
    } else {
      // For archive decrypt, we only have one file (the archive)
      if (inputPaths.value.length !== 1) {
        statusMessage.value = 'Please select exactly one archive file to decrypt';
        statusType.value = 'error';
        isProcessing.value = false;
        showProgress.value = false;
        stopProgressListener();
        return;
      }
      // inputPaths.value[0] is guaranteed to exist due to the check above
      const archivePath = inputPaths.value[0]!;
      result = await tauri.batchDecryptArchive(
        archivePath,
        outputDir.value,
        password.value,
        allowOverwrite,
        keyFilePath.value || undefined
      );
    }

    // Set status message
    if (result.success) {
      if (mode.value === 'encrypt') {
        statusMessage.value = `Successfully created encrypted archive with ${result.file_count} file${result.file_count !== 1 ? 's' : ''}`;
      } else {
        statusMessage.value = `Successfully extracted ${result.file_count} file${result.file_count !== 1 ? 's' : ''} from archive`;
      }
      statusType.value = 'success';
    } else {
      statusMessage.value = result.error ? sanitizeErrorMessage(result.error) : `Archive ${mode.value}ion failed`;
      statusType.value = 'error';
    }
  } catch (error) {
    statusMessage.value = sanitizeErrorMessage(error);
    statusType.value = 'error';
  } finally {
    // Always clear password, even when the operation fails
    password.value = '';
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
    keyFilePath.value = '';
    archiveName.value = '';
  }
}

// Switch batch mode
function switchBatchMode(newBatchMode: BatchMode) {
  if (batchMode.value !== newBatchMode) {
    batchMode.value = newBatchMode;
    inputPaths.value = [];
    batchResult.value = null;
    statusMessage.value = '';
    archiveName.value = '';
  }
}

// Get phase label for progress display
function getPhaseLabel(phase: string): string {
  switch (phase) {
    case 'archiving': return 'Creating archive';
    case 'encrypting': return 'Encrypting';
    case 'decrypting': return 'Decrypting';
    case 'extracting': return 'Extracting files';
    case 'complete': return 'Complete';
    default: return phase;
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
        Drop files here to add to batch
      </div>

      <!-- Mode Toggle -->
      <NButtonGroup class="mode-toggle">
        <NButton
          :type="mode === 'encrypt' ? 'primary' : 'default'"
          :ghost="mode !== 'encrypt'"
          :disabled="isProcessing"
          @click="switchMode('encrypt')"
        >
          Encrypt
        </NButton>
        <NButton
          :type="mode === 'decrypt' ? 'primary' : 'default'"
          :ghost="mode !== 'decrypt'"
          :disabled="isProcessing"
          @click="switchMode('decrypt')"
        >
          Decrypt
        </NButton>
      </NButtonGroup>

      <!-- Batch Mode Selector -->
      <div class="form-group batch-mode-selector">
        <label>Batch Mode:</label>
        <NRadioGroup :value="batchMode" @update:value="switchBatchMode" :disabled="isProcessing">
          <div class="batch-mode-options">
            <div class="batch-mode-option">
              <NRadio value="individual">Individual files</NRadio>
              <span class="radio-description">Each file encrypted separately</span>
            </div>
            <div class="batch-mode-option">
              <NRadio value="archive">Archive mode</NRadio>
              <span class="radio-description">Bundle into one encrypted archive</span>
            </div>
          </div>
        </NRadioGroup>
      </div>

      <!-- Compression Info Banner -->
      <NAlert v-if="mode === 'encrypt'" type="info" :show-icon="false" class="info-banner">
        <template v-if="batchMode === 'archive'">
          Files will be bundled into a compressed TAR archive, then encrypted as a single file.
        </template>
        <template v-else>
          Compression is automatically enabled for batch operations.
          Files are compressed with ZSTD before encryption for optimal size reduction.
        </template>
      </NAlert>

      <!-- Archive Name (archive mode encrypt only) -->
      <div v-if="batchMode === 'archive' && mode === 'encrypt'" class="form-group">
        <label for="archive-name">Archive Name (optional):</label>
        <NInput
          :input-props="{ id: 'archive-name' }"
          v-model:value="archiveName"
          placeholder="Leave empty for auto-generated name (archive_YYYYMMDD_HHMMSS)"
          :disabled="isProcessing"
          :status="archiveNameError ? 'error' : undefined"
        />
        <p v-if="archiveNameError" class="error-text">
          {{ archiveNameError }}
        </p>
        <p v-else class="hint-text">
          Custom name for the archive (without extension). Defaults to timestamp-based name.
        </p>
      </div>

      <!-- File Selection -->
      <div class="form-group">
        <label>
          <template v-if="batchMode === 'archive' && mode === 'decrypt'">
            Archive to Decrypt:
          </template>
          <template v-else>
            {{ mode === 'encrypt' ? 'Files to Encrypt' : 'Files to Decrypt' }}:
          </template>
        </label>
        <div class="file-input-group">
          <div class="file-count-display">
            <template v-if="batchMode === 'archive' && mode === 'decrypt'">
              {{ fileCount === 1 ? '1 archive' : 'No archive' }} selected
            </template>
            <template v-else>
              {{ fileCount }} file{{ fileCount !== 1 ? 's' : '' }} selected
            </template>
          </div>
          <NButton
            type="primary"
            @click="handleSelectFiles"
            :disabled="isProcessing"
            :title="batchMode === 'archive' && mode === 'decrypt' ? 'Choose an encrypted archive' : 'Choose multiple files to process'"
          >
            Browse
          </NButton>
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
              class="file-error-msg selectable"
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
          <NInput
            :value="outputDir"
            readonly
            placeholder="Select output directory..."
          />
          <NButton
            @click="handleSelectOutputDir"
            :disabled="isProcessing"
            title="Choose the output folder"
          >
            Browse
          </NButton>
        </div>
      </div>

      <!-- Output Safety Options -->
      <div class="form-group">
        <NCheckbox
          v-model:checked="neverOverwrite"
          :disabled="isProcessing"
        >
          Never overwrite existing files (auto-rename on conflicts)
        </NCheckbox>
        <p class="hint-text">
          If a filename already exists, we'll save as "name (1)".
        </p>
      </div>

      <!-- Password Input -->
      <PasswordSection
        input-id="batch-password"
        v-model="password"
        :placeholder="mode === 'encrypt' ? 'Enter password (min 8 characters)' : 'Enter decryption password'"
        :disabled="isProcessing"
        :autocomplete="mode === 'encrypt' ? 'new-password' : 'current-password'"
        :show-strength-meter="mode === 'encrypt'"
        :strength="passwordStrength"
        :is-password-valid="isPasswordValid"
        :hint-text="mode === 'decrypt' ? 'Enter the password used to encrypt these files' : undefined"
      />

      <!-- Key File Section -->
      <KeyFileSection
        v-model="keyFilePath"
        :disabled="isProcessing"
        :show-generate="mode !== 'decrypt'"
      />

      <!-- Action Button -->
      <NButton
        type="primary"
        block
        strong
        class="action-btn"
        @click="confirmBatchOperation"
        :disabled="!isFormValid"
        :title="mode === 'encrypt' ? 'Encrypt all selected files' : 'Decrypt all selected files'"
      >
        <span v-if="isProcessing">
          <template v-if="batchMode === 'archive'">
            {{ mode === 'encrypt' ? 'Creating Archive' : 'Extracting Archive' }}...
          </template>
          <template v-else>
            {{ mode === 'encrypt' ? 'Encrypting' : 'Decrypting' }}...
          </template>
        </span>
        <span v-else>
          <template v-if="batchMode === 'archive'">
            <template v-if="mode === 'encrypt'">
              Create Encrypted Archive ({{ fileCount }} file{{ fileCount !== 1 ? 's' : '' }})
            </template>
            <template v-else>
              Decrypt &amp; Extract Archive
            </template>
          </template>
          <template v-else>
            {{ mode === 'encrypt' ? 'Encrypt' : 'Decrypt' }} {{ fileCount }} File{{ fileCount !== 1 ? 's' : '' }}
          </template>
        </span>
      </NButton>

      <!-- Progress Bar (Individual Mode) -->
      <div v-if="showProgress && batchMode === 'individual' && batchProgress" class="progress-container">
        <div class="progress-bar-bg" role="progressbar" aria-label="Batch operation progress" :aria-valuenow="batchProgress.percent" aria-valuemin="0" aria-valuemax="100">
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

      <!-- Progress Bar (Archive Mode) -->
      <div v-if="showProgress && batchMode === 'archive' && archiveProgress" class="progress-container">
        <div class="progress-bar-bg" role="progressbar" aria-label="Batch operation progress" :aria-valuenow="archiveProgress.percent" aria-valuemin="0" aria-valuemax="100">
          <div
            class="progress-bar-fill"
            :style="{ width: `${archiveProgress.percent}%` }"
          ></div>
        </div>
        <div class="progress-info">
          <span class="progress-message">
            {{ getPhaseLabel(archiveProgress.phase) }}
            <template v-if="archiveProgress.current_file">
              : {{ archiveProgress.current_file }}
            </template>
          </span>
          <span class="progress-percent">
            {{ archiveProgress.percent }}%
          </span>
        </div>
      </div>

      <!-- Status Message -->
      <StatusMessage
        v-if="statusMessage"
        :message="statusMessage"
        :type="statusType"
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

/* Mode Toggle */
.mode-toggle {
  margin-bottom: 16px;
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
  cursor: default;
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
  cursor: default;
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
  margin-bottom: 16px;
}

.action-btn {
  margin-top: 8px;
}

/* Batch Mode Selector */
.batch-mode-options {
  display: flex;
  gap: 32px;
}

.batch-mode-option {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.radio-description {
  font-size: 12px;
  color: var(--muted);
  margin-left: 24px;
}
</style>
