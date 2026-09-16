import { useEffect, useRef } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { isNativeDesktop } from './storage.ts';

export async function requestAppClose(): Promise<void> {
  if (isNativeDesktop) await getCurrentWindow().close();
}

export async function destroyAppWindow(): Promise<void> {
  if (isNativeDesktop) await getCurrentWindow().destroy();
}

/** Route every native close request through the app's Save / Discard / Cancel overlay. */
export function useCloseProtection(requestClose: () => void): void {
  const current = useRef(requestClose);
  current.current = requestClose;

  useEffect(() => {
    if (!isNativeDesktop) return;

    const appWindow = getCurrentWindow();
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void appWindow.onCloseRequested(event => {
      event.preventDefault();
      current.current();
    }).then(removeListener => {
      if (disposed) removeListener();
      else unlisten = removeListener;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);
}
