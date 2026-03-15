import { invokeCommand } from './api';
import type { ScanInput, ScanResult, DesensitizeInput, DesensitizeResult, RestoreInput, RestoreResult } from '../types/session';

export interface NerStatus {
  ready: boolean;
  loading: boolean;
  error: string | null;
}

export const desensitizeApi = {
  scan: (input: ScanInput) =>
    invokeCommand<ScanResult>('desensitize_scan', { input }),
  execute: (input: DesensitizeInput) =>
    invokeCommand<DesensitizeResult>('desensitize_execute', { input }),
  restore: (input: RestoreInput) =>
    invokeCommand<RestoreResult>('restore_execute', { input }),
  getNerStatus: () =>
    invokeCommand<NerStatus>('ner_get_status'),
};
