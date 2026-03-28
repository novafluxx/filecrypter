<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue';
import Button from 'primevue/button';
import { openUrl } from '@tauri-apps/plugin-opener';
import { FILECRYPTER_DOWNLOAD_URL } from '../constants';
import { useSettings } from '../composables/useSettings';
import { copyTextToClipboard } from '../utils/clipboard';

const props = defineProps<{
  encryptedFilePath: string;
  usesKeyFile: boolean;
}>();

const settings = useSettings();
const copyState = ref<'idle' | 'success' | 'error'>('idle');
let copyStateTimeoutId: ReturnType<typeof window.setTimeout> | null = null;

const encryptedFileName = computed(() => props.encryptedFilePath.split(/[/\\]/).at(-1) ?? props.encryptedFilePath);

const shareInstructions = computed(() => {
  const steps = [
    'I shared an encrypted file with you using FileCrypter.',
    `File: ${encryptedFileName.value}`,
    '',
    'To open it:',
    `1. Download FileCrypter: ${FILECRYPTER_DOWNLOAD_URL}`,
    '2. Open the Decrypt tab and choose the encrypted file.',
    '3. Enter the password I send separately.',
  ];

  if (props.usesKeyFile) {
    steps.push('4. Also choose the matching key file I send separately.');
  }

  steps.push('');
  steps.push('Security tip: keep the encrypted file, password, and key file in separate messages.');

  return steps.join('\n');
});

function setCopyFeedback(state: 'success' | 'error') {
  copyState.value = state;

  if (copyStateTimeoutId !== null) {
    window.clearTimeout(copyStateTimeoutId);
  }

  copyStateTimeoutId = window.setTimeout(() => {
    copyState.value = 'idle';
    copyStateTimeoutId = null;
  }, 4000);
}

async function handleCopyInstructions() {
  const copied = await copyTextToClipboard(shareInstructions.value);
  setCopyFeedback(copied ? 'success' : 'error');

  if (copied) {
    await settings.trackShareKitCopied();
  }
}

async function handleOpenDownloadPage() {
  try {
    await openUrl(FILECRYPTER_DOWNLOAD_URL);
    await settings.trackShareKitDownloadOpened();
  } catch (error) {
    console.warn('Failed to open FileCrypter download page:', error);
  }
}

onBeforeUnmount(() => {
  if (copyStateTimeoutId !== null) {
    window.clearTimeout(copyStateTimeoutId);
  }
});
</script>

<template>
  <div class="share-kit">
    <div class="share-kit-header">
      <div>
        <h3 class="share-kit-title">Recipient Share Kit</h3>
        <p class="share-kit-subtitle">
          Encryption succeeded. Copy a ready-to-send message so the recipient knows how to decrypt
          <span class="selectable">{{ encryptedFileName }}</span>.
        </p>
      </div>
      <div class="share-kit-actions">
        <Button @click="handleCopyInstructions" label="Copy Instructions" />
        <Button outlined severity="secondary" @click="handleOpenDownloadPage" label="Open Download Page" />
      </div>
    </div>

    <pre class="share-kit-preview selectable">{{ shareInstructions }}</pre>

    <p v-if="copyState === 'success'" class="share-kit-feedback success-text">
      Recipient instructions copied to your clipboard.
    </p>
    <p v-else-if="copyState === 'error'" class="share-kit-feedback error-text">
      Clipboard access failed. You can still copy the instructions from the preview above.
    </p>
  </div>
</template>

<style scoped>
.share-kit {
  margin-top: 16px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--panel-alt);
  padding: 16px;
}

.share-kit-header {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  align-items: flex-start;
  margin-bottom: 12px;
  flex-wrap: wrap;
}

.share-kit-title {
  margin: 0 0 4px 0;
  font-size: 16px;
  color: var(--text);
}

.share-kit-subtitle {
  margin: 0;
  color: var(--muted);
  font-size: 13px;
}

.share-kit-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.share-kit-preview {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-word;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 12px;
  font-family: inherit;
  font-size: 13px;
  color: var(--text);
}

.share-kit-feedback {
  margin-top: 8px;
  margin-bottom: 0;
}

.success-text {
  color: var(--success-text);
}
</style>
