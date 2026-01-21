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
import { ref, onMounted, watch } from 'vue';
import { NButton, NCheckbox, NInput } from 'naive-ui';
import { useFileOps } from '../composables/useFileOps';
import { useTauri } from '../composables/useTauri';
import { usePasswordStrength } from '../composables/usePasswordStrength';
import { useProgress } from '../composables/useProgress';
import { useDragDrop } from '../composables/useDragDrop';
import { useSettings } from '../composables/useSettings';
import PasswordStrengthMeter from './PasswordStrengthMeter.vue';
import ProgressBar from './ProgressBar.vue';
import StatusMessage from './StatusMessage.vue';

// Initialize composables
// These provide reactive state and methods for file operations
const fileOps = useFileOps();
const tauri = useTauri();
const settings = useSettings();

// Apply default settings when initialized and sync when they change
watch(
  () => settings.isInitialized.value,
  (initialized) => {
    if (initialized) {
      fileOps.compressionEnabled.value = settings.defaultCompression.value;
      fileOps.neverOverwrite.value = settings.defaultNeverOverwrite.value;
    }
  },
  { immediate: true }
);

// Sync settings changes from Settings tab to this tab
watch(
  () => settings.defaultCompression.value,
  (newValue) => {
    if (settings.isInitialized.value) {
      fileOps.compressionEnabled.value = newValue;
    }
  }
);

watch(
  () => settings.defaultNeverOverwrite.value,
  (newValue) => {
    if (settings.isInitialized.value) {
      fileOps.neverOverwrite.value = newValue;
    }
  }
);

// Password strength analysis
// Provides reactive feedback as user types their password
const { strength: passwordStrength } = usePasswordStrength(fileOps.password);

// Progress tracking for encryption operation
const { progress, isActive: showProgress, startListening, stopListening } = useProgress();

// Drop zone element reference for drag-and-drop
const dropZoneRef = ref<HTMLElement>();

// Drag-and-drop file handling
const { isDragging, handleDragOver, handleDragLeave, handleDrop, setupDragDrop } = useDragDrop(
  (paths) => {
    // For single file encryption, use the first dropped file
    const path = paths[0];
    if (path) {
      fileOps.setInputPath(path, true); // true = encryption mode

      // If a default output directory is set, use it instead of the input file's directory
      const defaultDir = settings.defaultOutputDirectory.value;
      if (defaultDir) {
        const filename = path.split(/[/\\]/).pop() ?? '';
        fileOps.setOutputPath(`${defaultDir}/${filename}.encrypted`);
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
 * Handle file selection for encryption
 *
 * Opens native file picker, then auto-suggests output filename
 */
async function handleSelectFile() {
  const path = await tauri.selectFile('Select File to Encrypt');

  if (path) {
    // setInputPath automatically suggests output path with .encrypted extension
    fileOps.setInputPath(path, true); // true = encryption mode

    // If a default output directory is set, use it instead of the input file's directory
    const defaultDir = settings.defaultOutputDirectory.value;
    if (defaultDir) {
      const filename = path.split(/[/\\]/).pop() ?? '';
      fileOps.setOutputPath(`${defaultDir}/${filename}.encrypted`);
    }
  }
}

/**
 * Handle click on the readonly input
 *
 * Mirrors the Browse button behavior for easier file selection
 */
async function handleSelectFileInputClick() {
  if (fileOps.isProcessing.value) {
    return;
  }

  await handleSelectFile();
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
        Drop file here to encrypt
      </div>

      <!-- File Input Section -->
    <div class="form-group">
      <label>File to Encrypt:</label>
      <div class="file-input-group">
        <NInput
          :input-props="{ id: 'encrypt-input', title: 'Click to choose a file' }"
          :value="fileOps.inputPath.value"
          readonly
          class="clickable-input"
          placeholder="Select or drag a file..."
          @click="handleSelectFileInputClick"
        />
        <NButton
          type="primary"
          @click="handleSelectFile"
          :disabled="fileOps.isProcessing.value"
          title="Choose a file to encrypt"
        >
          Browse
        </NButton>
      </div>
    </div>

    <!-- Output Path Section -->
    <div class="form-group">
      <label for="encrypt-output">Save Encrypted File As:</label>
      <div class="file-input-group">
        <NInput
          :input-props="{ id: 'encrypt-output' }"
          :value="fileOps.outputPath.value"
          readonly
          placeholder="Will auto-generate from input filename..."
        />
        <NButton
          @click="handleSelectOutput"
          :disabled="fileOps.isProcessing.value"
          title="Choose where to save the encrypted file"
        >
          Change
        </NButton>
      </div>
    </div>

    <!-- Output Safety Options -->
    <div class="form-group">
      <NCheckbox
        v-model:checked="fileOps.neverOverwrite.value"
        :disabled="fileOps.isProcessing.value"
      >
        Never overwrite existing files (auto-rename on conflicts)
      </NCheckbox>
      <p class="hint-text">
        If the output name already exists, we'll save as "name (1)".
      </p>
    </div>

    <!-- Compression Option -->
    <div class="form-group">
      <NCheckbox
        v-model:checked="fileOps.compressionEnabled.value"
        :disabled="fileOps.isProcessing.value"
      >
        Enable compression (ZSTD)
      </NCheckbox>
      <p class="hint-text">
        Compresses file before encryption. Reduces size by ~70% for text/documents,
        less for images/videos. Slightly slower encryption.
      </p>
    </div>

    <!-- Password Input Section -->
    <div class="form-group">
      <label for="encrypt-password">Password:</label>
      <NInput
        :input-props="{ id: 'encrypt-password' }"
        type="password"
        show-password-on="click"
        :value="fileOps.password.value"
        @update:value="fileOps.setPassword"
        placeholder="Enter password (min 8 characters)"
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
    <NButton
      type="primary"
      block
      strong
      class="action-btn"
      @click="handleEncrypt"
      :disabled="!fileOps.isFormValid.value"
      title="Start encrypting with the selected file and password"
    >
      <span v-if="fileOps.isProcessing.value">Encrypting...</span>
      <span v-else>Encrypt File</span>
    </NButton>

    <!-- Progress Bar (shown during encryption) -->
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

.clickable-input :deep(input) {
  cursor: pointer;
}
</style>
