/**
 * Copy text to the clipboard with a DOM fallback for desktop webviews.
 */
export async function copyTextToClipboard(text: string): Promise<boolean> {
  try {
    if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch (error) {
    console.warn('Clipboard API failed, falling back to execCommand:', error);
  }

  try {
    const textArea = document.createElement('textarea');
    textArea.value = text;
    textArea.setAttribute('readonly', 'true');
    textArea.style.position = 'fixed';
    textArea.style.opacity = '0';
    textArea.style.pointerEvents = 'none';

    document.body.appendChild(textArea);
    textArea.select();
    textArea.setSelectionRange(0, text.length);

    const copied = document.execCommand('copy');
    document.body.removeChild(textArea);
    return copied;
  } catch (error) {
    console.warn('Fallback clipboard copy failed:', error);
    return false;
  }
}
