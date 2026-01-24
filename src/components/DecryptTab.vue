<!-- components/DecryptTab.vue - File Decryption Interface -->
<!--
  This component provides the UI for decrypting files.
  Uses the shared CryptoOperationForm component with decrypt-specific options.
-->

<script setup lang="ts">
import CryptoOperationForm from './CryptoOperationForm.vue';
</script>

<template>
  <CryptoOperationForm
    mode="decrypt"
    input-label="Encrypted File:"
    input-placeholder="Select or drag a .encrypted file..."
    input-id="decrypt-input"
    output-label="Save Decrypted File As:"
    output-placeholder="Will auto-generate from encrypted filename..."
    output-id="decrypt-output"
    password-id="decrypt-password"
    password-placeholder="Enter decryption password"
    action-button-text="Decrypt File"
    processing-button-text="Decrypting..."
    drop-overlay-text="Drop encrypted file here"
    password-hint-text="Enter the password used to encrypt this file"
  />
</template>
