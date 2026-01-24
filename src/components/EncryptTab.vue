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
import { ref, onMounted } from 'vue';
import { NButton, NCheckbox, NInput } from 'naive-ui';
import { join } from '@tauri-apps/api/path';
import { useFileOps } from '../composables/useFileOps';
import { useTauri } from '../composables/useTauri';
import { usePasswordStrength } from '../composables/usePasswordStrength';
import { useProgress } from '../composables/useProgress';
import { useDragDrop } from '../composables/useDragDrop';
import { useSettings } from '../composables/useSettings';
import { useSettingsSync } from '../composables/useSettingsSync';
import OverwriteCheckbox from './OverwriteCheckbox.vue';
import PasswordSection from './PasswordSection.vue';
import ProgressBar from './ProgressBar.vue';
import StatusMessage from './StatusMessage.vue';

// Initialize composables
// These provide reactive state and methods for file operations
const fileOps = useFileOps();
const tauri = useTauri();
const settings = useSettings();

// Sync settings to fileOps state (initial + reactive updates)
useSettingsSync(settings, {
  compression: fileOps.compressionEnabled,
  neverOverwrite: fileOps.neverOverwrite,
});

// Password strength analysis
// Provides reactive feedback as user types their password
const { strength: passwordStrength } = usePasswordStrength(fileOps.password);

// Progress tracking for encryption operation
const { progress, isActive: showProgress, startListening, stopListening } = useProgress();

// Drop zone element reference for drag-and-drop
const dropZoneRef = ref<HTMLElement>();

// Drag-and-drop file handling
const { isDragging, handleDragOver, handleDragLeave, handleDrop, setupDragDrop } = useDragDrop(
  async (paths) => {
    // For single file encryption, use the first dropped file
    const path = paths[0];
    if (path) {
      fileOps.setInputPath(path, true); // true = encryption mode

      // If a default output directory is set, use it instead of the input file's directory
      const defaultDir = settings.defaultOutputDirectory.value;
      if (defaultDir) {
        const filename = path.split(/[/\\]/).pop() ?? '';
        const outputPath = await join(defaultDir, `${filename}.encrypted`);
        fileOps.setOutputPath(outputPath);
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
      const outputPath = await join(defaultDir, `${filename}.encrypted`);
      fileOps.setOutputPath(outputPath);
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
        <label for="encrypt-input">File to Encrypt:</label>
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
      <OverwriteCheckbox
        v-model="fileOps.neverOverwrite.value"
        :disabled="fileOps.isProcessing.value"
      />

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
      <PasswordSection
        input-id="encrypt-password"
        v-model="fileOps.password.value"
        placeholder="Enter password (min 8 characters)"
        :disabled="fileOps.isProcessing.value"
        show-strength-meter
        :strength="passwordStrength"
        :is-password-valid="fileOps.isPasswordValid.value"
      />

      <!-- Encrypt Button -->
      <NButton
        type="primary"
        block
        strong
        class="action-btn"
        @click="handleEncrypt"
        :disabled="!fileOps.isEncryptFormValid.value"
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
