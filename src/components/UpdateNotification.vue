<!-- UpdateNotification.vue - Update Available Notification Banner -->
<!--
  Displays a notification banner when a new app version is available.
  Provides buttons to update now or dismiss the notification.

  Desktop-only: Updates are handled by app stores on mobile platforms.
-->

<script setup lang="ts">
import { NButton, NAlert, NProgress } from 'naive-ui';
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
</script>

<template>
  <Transition name="slide-down">
    <div v-if="updateAvailable" class="update-notification">
      <NAlert type="info" :show-icon="false" class="update-alert">
        <div class="update-content">
          <div class="update-text">
            <strong>Update Available</strong>
            <span v-if="updateVersion">Version {{ updateVersion }} is ready to install.</span>
          </div>
          <div v-if="error" class="update-error">
            <span class="update-error-text">{{ error }}</span>
            <NButton size="small" type="primary" @click="downloadAndInstall">
              Retry
            </NButton>
            <NButton size="small" quaternary @click="dismissUpdate">
              Dismiss
            </NButton>
          </div>
          <div v-else-if="isDownloading" class="update-progress">
            <NProgress
              type="line"
              :percentage="downloadProgress"
              :show-indicator="false"
              status="info"
            />
            <span class="progress-text">Downloading...</span>
          </div>
          <div v-else class="update-actions">
            <NButton size="small" type="primary" @click="downloadAndInstall">
              Update Now
            </NButton>
            <NButton size="small" quaternary @click="dismissUpdate">
              Later
            </NButton>
          </div>
        </div>
      </NAlert>
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
