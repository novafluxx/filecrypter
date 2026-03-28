<!-- components/EncryptTab.vue - File Encryption Interface -->
<!--
  This component provides the UI for encrypting files.
  Uses the shared CryptoOperationForm component with encrypt-specific options.
-->

<script setup lang="ts">
import Checkbox from 'primevue/checkbox';
import CryptoOperationForm from './CryptoOperationForm.vue';
import RecipientShareKit from './RecipientShareKit.vue';
</script>

<template>
  <CryptoOperationForm
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
  >
    <template #after-overwrite="{ fileOps }">
      <!-- Compression Option -->
      <div class="form-group">
        <div class="checkbox-field">
          <Checkbox
            v-model="fileOps.compressionEnabled"
            :disabled="fileOps.isProcessing"
            :binary="true"
            inputId="compression-checkbox"
          />
          <label for="compression-checkbox">Enable compression (ZSTD)</label>
        </div>
        <p class="hint-text">
          Compresses file before encryption. Reduces size by ~70% for text/documents,
          less for images/videos. Slightly slower encryption.
        </p>
      </div>
    </template>

    <template #after-status="{ fileOps }">
      <RecipientShareKit
        v-if="fileOps.lastSuccessfulOutputPath"
        :encrypted-file-path="fileOps.lastSuccessfulOutputPath"
        :uses-key-file="fileOps.lastSuccessfulUsedKeyFile"
      />
    </template>
  </CryptoOperationForm>
</template>
