<!-- components/CryptoOperationForm.vue - Shared Crypto Operation Form -->
<!--
  This component provides the shared UI for encrypt and decrypt operations.
  It uses the useCryptoOperation composable and accepts props for mode-specific
  text labels and slots for mode-specific content.

  Props:
  - mode: 'encrypt' | 'decrypt'
  - inputLabel, inputPlaceholder: File input section labels
  - outputLabel, outputPlaceholder: Output path section labels
  - actionButtonText, processingButtonText: Button labels
  - dropOverlayText: Text shown when dragging files

  Slots:
  - #after-overwrite: Content after overwrite checkbox (e.g., compression checkbox)
  - #password-section: Complete password section override
-->

<script setup lang="ts">
import { NButton, NInput } from 'naive-ui';
import { useCryptoOperation } from '../composables/useCryptoOperation';
import type { PasswordStrength } from '../composables/usePasswordStrength';
import KeyFileSection from './KeyFileSection.vue';
import OverwriteCheckbox from './OverwriteCheckbox.vue';
import PasswordSection from './PasswordSection.vue';
import ProgressBar from './ProgressBar.vue';
import StatusMessage from './StatusMessage.vue';

interface Props {
  mode: 'encrypt' | 'decrypt';
  inputLabel: string;
  inputPlaceholder: string;
  inputId: string;
  outputLabel: string;
  outputPlaceholder: string;
  outputId: string;
  passwordId: string;
  passwordPlaceholder: string;
  actionButtonText: string;
  processingButtonText: string;
  dropOverlayText: string;
  // Password section props
  showStrengthMeter?: boolean;
  passwordStrength?: PasswordStrength;
  passwordHintText?: string;
  // Input clickable (encrypt only)
  inputClickable?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  showStrengthMeter: false,
  passwordStrength: undefined,
  passwordHintText: undefined,
  inputClickable: false,
});

// Initialize the unified composable
const {
  fileOps,
  progress,
  showProgress,
  isDragging,
  dropZoneRef,
  isFormValid,
  handleSelectFile,
  handleSelectOutput,
  handleOperation,
  handleSelectFileInputClick,
  handleDragOver,
  handleDragLeave,
  handleDrop,
} = useCryptoOperation({ mode: props.mode });

// Expose fileOps for parent components that need access (e.g., for password strength)
defineExpose({
  fileOps,
});
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
        {{ dropOverlayText }}
      </div>

      <!-- File Input Section -->
      <div class="form-group">
        <label :for="inputId">{{ inputLabel }}</label>
        <div class="file-input-group">
          <NInput
            :input-props="{ id: inputId, title: inputClickable ? 'Click to choose a file' : undefined }"
            :value="fileOps.inputPath.value"
            readonly
            :class="{ 'clickable-input': inputClickable }"
            :placeholder="inputPlaceholder"
            @click="inputClickable ? handleSelectFileInputClick() : undefined"
          />
          <NButton
            type="primary"
            @click="handleSelectFile"
            :disabled="fileOps.isProcessing.value"
            :title="`Choose a file to ${mode}`"
          >
            Browse
          </NButton>
        </div>
      </div>

      <!-- Output Path Section -->
      <div class="form-group">
        <label :for="outputId">{{ outputLabel }}</label>
        <div class="file-input-group">
          <NInput
            :input-props="{ id: outputId }"
            :value="fileOps.outputPath.value"
            readonly
            :placeholder="outputPlaceholder"
          />
          <NButton
            @click="handleSelectOutput"
            :disabled="fileOps.isProcessing.value"
            :title="`Choose where to save the ${mode}ed file`"
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

      <!-- Slot for mode-specific content after overwrite checkbox -->
      <slot name="after-overwrite" :fileOps="fileOps" />

      <!-- Password Input Section -->
      <PasswordSection
        :input-id="passwordId"
        v-model="fileOps.password.value"
        :placeholder="passwordPlaceholder"
        :disabled="fileOps.isProcessing.value"
        :show-strength-meter="showStrengthMeter"
        :strength="passwordStrength"
        :is-password-valid="fileOps.isPasswordValid.value"
        :hint-text="passwordHintText"
      />

      <!-- Key File Section -->
      <KeyFileSection
        v-model="fileOps.keyFilePath.value"
        :disabled="fileOps.isProcessing.value"
        :show-generate="mode !== 'decrypt'"
      />

      <!-- Action Button -->
      <NButton
        type="primary"
        block
        strong
        class="action-btn"
        @click="handleOperation"
        :disabled="!isFormValid"
        :title="`Start ${mode}ing with the selected file and password`"
      >
        <span v-if="fileOps.isProcessing.value">{{ processingButtonText }}</span>
        <span v-else>{{ actionButtonText }}</span>
      </NButton>

      <!-- Progress Bar -->
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
