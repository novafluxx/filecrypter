<!-- components/EncryptTab.vue - File Encryption Interface -->
<!--
  This component provides the UI for encrypting files.
  Uses the shared CryptoOperationForm component with encrypt-specific options.
-->

<script setup lang="ts">
import { ref, computed } from 'vue';
import { NCheckbox } from 'naive-ui';
import CryptoOperationForm from './CryptoOperationForm.vue';
import { usePasswordStrength } from '../composables/usePasswordStrength';

// Reference to the form component to access fileOps
const formRef = ref<InstanceType<typeof CryptoOperationForm>>();

// Password strength analysis (needs access to password from form)
const password = computed(() => formRef.value?.fileOps?.password.value ?? '');
const { strength: passwordStrength } = usePasswordStrength(password);
</script>

<template>
  <CryptoOperationForm
    ref="formRef"
    mode="encrypt"
    input-label="File to Encrypt:"
    input-placeholder="Select or drag a file..."
    input-id="encrypt-input"
    output-label="Save Encrypted File As:"
    output-placeholder="Will auto-generate from input filename..."
    output-id="encrypt-output"
    password-id="encrypt-password"
    password-placeholder="Enter password (min 8 characters)"
    action-button-text="Encrypt File"
    processing-button-text="Encrypting..."
    drop-overlay-text="Drop file here to encrypt"
    :show-strength-meter="true"
    :password-strength="passwordStrength"
  >
    <template #after-overwrite="{ fileOps }">
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
    </template>
  </CryptoOperationForm>
</template>
