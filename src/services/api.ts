import { invoke } from '@tauri-apps/api/core';

export async function invokeCommand<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (error) {
    console.error(`Command ${cmd} failed:`, error);
    throw error;
  }
}
