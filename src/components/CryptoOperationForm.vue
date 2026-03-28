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
import { reactive, computed } from 'vue';
import Button from 'primevue/button';
import InputText from 'primevue/inputtext';
import { useCryptoOperation } from '../composables/useCryptoOperation';
import { usePasswordStrength, type PasswordStrength } from '../composables/usePasswordStrength';
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
  passwordHintText?: string;
}

const props = withDefaults(defineProps<Props>(), {
  passwordHintText: undefined,
});

// Initialize the unified composable
const {
  fileOps: rawFileOps,
  progress,
  showProgress,
  isDragging,
  dropZoneRef,
  isFormValid,
  handleSelectFile,
  handleSelectOutput,
  handleOperation,
  handleDragOver,
  handleDragLeave,
  handleDrop,
} = useCryptoOperation({ mode: props.mode });

// Wrap in reactive() so Vue auto-unwraps nested refs in templates,
// allowing fileOps.inputPath instead of fileOps.inputPath.value
const fileOps = reactive(rawFileOps);

const isEncrypt = computed(() => props.mode === 'encrypt');

// Always call composable unconditionally (Vue composable rules), but guard the
// expensive computation so it only runs in encrypt mode.
const { strength: rawStrength } = usePasswordStrength(rawFileOps.password);
const passwordStrength = computed<PasswordStrength>(() =>
  isEncrypt.value ? rawStrength.value : { score: 0, level: 'weak', feedback: [] },
);
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
          <InputText
            :id="inputId"
            :modelValue="fileOps.inputPath"
            readonly
            :placeholder="inputPlaceholder"
            fluid
          />
          <Button
            @click="handleSelectFile"
            :disabled="fileOps.isProcessing"
            :title="`Choose a file to ${mode}`"
            label="Browse"
          />
        </div>
      </div>

      <!-- Output Path Section -->
      <div class="form-group">
        <label :for="outputId">{{ outputLabel }}</label>
        <div class="file-input-group">
          <InputText
            :id="outputId"
            :modelValue="fileOps.outputPath"
            readonly
            :placeholder="outputPlaceholder"
            fluid
          />
          <Button
            @click="handleSelectOutput"
            :disabled="fileOps.isProcessing"
            :title="`Choose where to save the ${mode}ed file`"
            label="Change"
          />
        </div>
      </div>

      <!-- Output Safety Options -->
      <OverwriteCheckbox
        v-model="fileOps.neverOverwrite"
        :disabled="fileOps.isProcessing"
        :input-id="`${mode}-overwrite`"
      />

      <!-- Slot for mode-specific content after overwrite checkbox -->
      <slot name="after-overwrite" :fileOps="fileOps" />

      <!-- Password Input Section -->
      <PasswordSection
        :input-id="passwordId"
        v-model="fileOps.password"
        :placeholder="passwordPlaceholder"
        :disabled="fileOps.isProcessing"
        :autocomplete="isEncrypt ? 'new-password' : 'current-password'"
        :show-strength-meter="isEncrypt"
        :strength="passwordStrength"
        :is-password-valid="fileOps.isPasswordValid"
        :hint-text="passwordHintText"
      />

      <!-- Key File Section -->
      <KeyFileSection
        v-model="fileOps.keyFilePath"
        :disabled="fileOps.isProcessing"
        :show-generate="mode !== 'decrypt'"
      />

      <!-- Action Button -->
      <Button
        class="action-btn"
        @click="handleOperation"
        :disabled="!isFormValid"
        :title="`Start ${mode}ing with the selected file and password`"
        fluid
      >
        <span v-if="fileOps.isProcessing">{{ processingButtonText }}</span>
        <span v-else>{{ actionButtonText }}</span>
      </Button>

      <!-- Progress Bar -->
      <ProgressBar
        v-if="showProgress && progress"
        :percent="progress.percent"
        :message="progress.message"
      />

      <!-- Status Message -->
      <StatusMessage
        v-if="fileOps.statusMessage"
        :message="fileOps.statusMessage"
        :type="fileOps.statusType"
      />

      <slot name="after-status" :fileOps="fileOps" />
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
</style>
