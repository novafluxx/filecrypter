<!-- components/ChangelogAction.vue - Desktop header action to open repository changelog -->
<script setup lang="ts">
import Button from 'primevue/button';
import { useConfirm } from 'primevue/useconfirm';
import { openUrl } from '@tauri-apps/plugin-opener';

const confirm = useConfirm();

const CHANGELOG_URL = 'https://github.com/novafluxx/filecrypter/blob/main/CHANGELOG.md';

function handleOpenChangelog() {
  confirm.require({
    header: 'Open Changelog',
    message: 'This will open the changelog in your default browser.',
    acceptLabel: 'Continue',
    rejectLabel: 'No',
    accept: async () => {
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
  <Button
    text
    size="small"
    title="Open project changelog in your default browser"
    @click="handleOpenChangelog"
    label="Changelog"
  />
</template>
