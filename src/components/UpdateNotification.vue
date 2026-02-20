<!-- UpdateNotification.vue - Update Available Notification Banner -->
<!--
  Displays a notification banner when a new app version is available.
  Provides buttons to update now or dismiss the notification.

  Desktop-only: Updates are handled by app stores on mobile platforms.
-->

<script setup lang="ts">
import Button from 'primevue/button';
import Message from 'primevue/message';
import ProgressBar from 'primevue/progressbar';
import { openUrl } from '@tauri-apps/plugin-opener';
import { useUpdater } from '../composables/useUpdater';

const {
  updateAvailable,
  updateVersion,
  isDownloading,
  downloadProgress,
  error,
  downloadAndInstall,
  dismissUpdate,
} = useUpdater();

function openReleaseNotes() {
  if (updateVersion.value) {
    openUrl(`https://github.com/novafluxx/filecrypter/releases/tag/v${updateVersion.value}`);
  }
}
</script>

<template>
  <Transition name="slide-down">
    <div v-if="updateAvailable" class="update-notification">
      <Message severity="info" :closable="false" class="update-alert">
        <div class="update-content">
          <div class="update-text">
            <strong>Update Available</strong>
            <span v-if="updateVersion">
              Version {{ updateVersion }} is ready to install.
              <a class="whats-new-link" @click.prevent="openReleaseNotes">What's new</a>
            </span>
          </div>
          <div v-if="error" class="update-error">
            <span class="update-error-text">{{ error }}</span>
            <Button size="small" @click="downloadAndInstall" label="Retry" />
            <Button size="small" text @click="dismissUpdate" label="Dismiss" />
          </div>
          <div v-else-if="isDownloading" class="update-progress">
            <ProgressBar
              :value="downloadProgress"
              :showValue="false"
              class="update-progress-bar"
            />
            <span class="progress-text">Downloading...</span>
          </div>
          <div v-else class="update-actions">
            <Button size="small" @click="downloadAndInstall" label="Update Now" />
            <Button size="small" text @click="dismissUpdate" label="Later" />
          </div>
        </div>
      </Message>
    </div>
  </Transition>
</template>

<style scoped>
.update-notification {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  z-index: 1000;
  padding: 8px 16px;
  background: var(--bg);
}

.update-alert {
  max-width: 600px;
  margin: 0 auto;
}

.update-content {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
}

.update-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.update-text strong {
  font-size: 14px;
}

.update-text span {
  font-size: 12px;
  color: var(--muted);
}

.whats-new-link {
  color: var(--primary);
  cursor: pointer;
  text-decoration: underline;
  margin-left: 4px;
}

.update-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.update-progress {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 150px;
}

.update-progress-bar {
  height: 6px;
  flex: 1;
}

.progress-text {
  font-size: 12px;
  color: var(--muted);
  white-space: nowrap;
}

.update-error {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.update-error-text {
  font-size: 12px;
  color: var(--error, #d03050);
}

/* Slide down animation */
.slide-down-enter-active,
.slide-down-leave-active {
  transition: transform 0.3s ease, opacity 0.3s ease;
}

.slide-down-enter-from,
.slide-down-leave-to {
  transform: translateY(-100%);
  opacity: 0;
}
</style>
