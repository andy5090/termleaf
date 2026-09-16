import { isTauri } from '@tauri-apps/api/core';

export const isNativeDesktop = isTauri();
