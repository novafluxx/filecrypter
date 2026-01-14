<!-- components/HelpTab.vue - User Guide Help Tab -->
<!--
  This component renders the README_USER.md content as in-app help.

  The markdown file is bundled at build time via Vite's ?raw import.
-->

<script setup lang="ts">
import { marked } from 'marked';
import helpMarkdown from '../../README_USER.md?raw';

const helpHtml = marked.parse(helpMarkdown) as string;
</script>

<template>
  <div class="tab-content">
    <div class="content-panel help-content selectable" v-html="helpHtml"></div>
  </div>
</template>

<style scoped>
/* Component-specific styles - shared styles are in src/shared.css */

.tab-content {
  padding: 16px;
  max-width: 800px;
  margin: 0 auto;
  position: relative;
}

.content-panel {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px;
  position: relative;
}

.help-content :deep(h1) {
  font-size: 24px;
  margin-bottom: 12px;
}

.help-content :deep(h2) {
  font-size: 20px;
  margin-top: 16px;
  margin-bottom: 8px;
}

.help-content :deep(h3) {
  font-size: 18px;
  margin-top: 12px;
  margin-bottom: 6px;
}

.help-content :deep(p) {
  margin-bottom: 12px;
}

.help-content :deep(ul),
.help-content :deep(ol) {
  padding-left: 24px;
  margin-bottom: 12px;
}

.help-content :deep(li) {
  margin-bottom: 6px;
}

.help-content :deep(code) {
  background: var(--panel-alt);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 2px 4px;
  font-size: 0.9em;
}

.help-content :deep(pre) {
  background: var(--panel-alt);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 12px;
  overflow-x: auto;
  margin-bottom: 12px;
}

.help-content :deep(a) {
  color: var(--accent);
  text-decoration: underline;
}

.help-content :deep(hr) {
  border: none;
  border-top: 1px solid var(--border);
  margin: 16px 0;
}
</style>
