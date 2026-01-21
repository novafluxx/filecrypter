import { ref, onMounted } from 'vue';
import { getVersion } from '@tauri-apps/api/app';

/**
 * Composable to fetch and expose the app version from Tauri
 * @returns Reactive version string (empty on error or before mount)
 */
export function useVersion() {
  const version = ref<string>('');

  onMounted(async () => {
    try {
      version.value = await getVersion();
    } catch {
      version.value = '';
    }
  });

  return { version };
}
