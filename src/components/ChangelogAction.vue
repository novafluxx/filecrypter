<!-- components/ChangelogAction.vue - Desktop header action to open repository changelog -->
<script setup lang="ts">
import { NButton, useDialog } from 'naive-ui';
import { openUrl } from '@tauri-apps/plugin-opener';

const dialog = useDialog();

const CHANGELOG_URL = 'https://github.com/novafluxx/filecrypter/blob/main/CHANGELOG.md';

function handleOpenChangelog() {
  dialog.warning({
    title: 'Open Changelog',
    content: 'This will open the changelog in your default browser.',
    positiveText: 'Continue',
    negativeText: 'No',
    positiveButtonProps: {
      type: 'primary',
      ghost: false,
      size: 'medium',
    },
    negativeButtonProps: {
      type: 'primary',
      ghost: false,
      size: 'medium',
    },
    onAfterEnter() {
      const actionButtons = Array.from(
        document.querySelectorAll('.n-dialog__action .n-button')
      );
      const continueButton = actionButtons.find(
        (button) => button.textContent?.trim() === 'Continue'
      );
      if (continueButton instanceof HTMLElement) {
        continueButton.focus();
      }
    },
    async onPositiveClick() {
      try {
        await openUrl(CHANGELOG_URL);
      } catch (error) {
        console.warn('Failed to open changelog URL:', error);
      }
    },
  });
}
</script>

<template>
  <NButton
    quaternary
    size="small"
    title="Open project changelog in your default browser"
    @click="handleOpenChangelog"
  >
    Changelog
  </NButton>
</template>
