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
import { onMounted, watch } from 'vue';
import { useFileOps } from '../composables/useFileOps';
import { useTauri } from '../composables/useTauri';
import { usePasswordStrength } from '../composables/usePasswordStrength';
import { useProgress } from '../composables/useProgress';
import { useDragDrop } from '../composables/useDragDrop';
import { usePasswordVisibility } from '../composables/usePasswordVisibility';
import { useSettings } from '../composables/useSettings';
import PasswordStrengthMeter from './PasswordStrengthMeter.vue';
import ProgressBar from './ProgressBar.vue';
import IconEye from './icons/IconEye.vue';
import IconEyeOff from './icons/IconEyeOff.vue';

// Initialize composables
// These provide reactive state and methods for file operations
const fileOps = useFileOps();
const tauri = useTauri();
const settings = useSettings();

// Apply default settings when initialized
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

// Password visibility toggle
const { isPasswordVisible, togglePasswordVisibility } = usePasswordVisibility();

// Password strength analysis
// Provides reactive feedback as user types their password
const { strength: passwordStrength } = usePasswordStrength(fileOps.password);

// Progress tracking for encryption operation
const { progress, isActive: showProgress, startListening, stopListening } = useProgress();

// Drag-and-drop file handling
const { isDragging, handleDragOver, handleDragLeave, handleDrop, setupDragDrop } = useDragDrop(
  (path) => {
    fileOps.setInputPath(path, true); // true = encryption mode

    // If a default output directory is set, use it instead of the input file's directory
    const defaultDir = settings.defaultOutputDirectory.value;
    if (defaultDir) {
      const filename = path.split(/[/\\]/).pop() ?? '';
      fileOps.setOutputPath(`${defaultDir}/${filename}.encrypted`);
    }
  }
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
        <input
          id="encrypt-input"
          type="text"
          :value="fileOps.inputPath.value"
          readonly
          placeholder="Select or drag a file..."
          class="file-input"
          title="Drag a file here or click Browse to select one"
        />
        <button
          @click="handleSelectFile"
          class="btn btn-primary"
          :disabled="fileOps.isProcessing.value"
          title="Choose a file to encrypt"
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
          title="Auto-generated output path; click Change to pick a different location"
        />
        <button
          @click="handleSelectOutput"
          class="btn btn-secondary"
          :disabled="fileOps.isProcessing.value"
          title="Choose where to save the encrypted file"
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
          title="Prevent overwriting by auto-renaming on name conflicts"
        />
        Never overwrite existing files (auto-rename on conflicts)
      </label>
      <p class="hint-text">
        If the output name already exists, we'll save as "name (1)".
      </p>
    </div>

    <!-- Compression Option -->
    <div class="form-group">
      <label class="checkbox-row">
        <input
          type="checkbox"
          v-model="fileOps.compressionEnabled.value"
          :disabled="fileOps.isProcessing.value"
          title="Compress file before encryption to reduce size"
        />
        Enable compression (ZSTD)
      </label>
      <p class="hint-text">
        Compresses file before encryption. Reduces size by ~70% for text/documents,
        less for images/videos. Slightly slower encryption.
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
          title="Enter a strong password (at least 8 characters)"
        />
        <button
          type="button"
          class="password-toggle-btn"
          @click="togglePasswordVisibility"
          :disabled="fileOps.isProcessing.value"
          :aria-label="isPasswordVisible ? 'Hide password' : 'Show password'"
          :title="isPasswordVisible ? 'Hide password' : 'Show password'"
        >
          <IconEye v-if="!isPasswordVisible" />
          <IconEyeOff v-else />
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
      title="Start encrypting with the selected file and password"
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
        role="status"
        aria-live="polite"
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
