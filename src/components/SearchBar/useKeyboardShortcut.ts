import { useEffect } from 'react';

/**
 * Custom hook to listen for keyboard shortcuts
 * @param {string} key - The key to listen for (e.g., 'Space')
 * @param {boolean} ctrlKey - Whether Ctrl is required
 * @param {boolean} shiftKey - Whether Shift is required
 * @param {boolean} altKey - Whether Alt is required
 * @param {Function} callback - Function to execute when shortcut is pressed
 */
export const useKeyboardShortcut = (
  key: string,
  ctrlKey: boolean,
  shiftKey: boolean,
  altKey: boolean,
  callback: () => void
) => {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (
        e.key === key &&
        e.ctrlKey === ctrlKey &&
        e.shiftKey === shiftKey &&
        e.altKey === altKey
      ) {
        e.preventDefault();
        callback();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [key, ctrlKey, shiftKey, altKey, callback]);
};