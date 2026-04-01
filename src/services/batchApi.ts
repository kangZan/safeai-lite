import { invokeCommand } from './api';
import type {
  BatchScanInput,
  BatchScanResult,
  BatchExecuteInput,
  BatchExecuteResult,
  BatchExportInput,
  BatchFileResult,
  BatchSessionListItem,
  FolderFileEntry,
} from '../types/batch';

export const batchApi = {
  /** 递归扫描文件夹，返回支持格式的文件条目（不读取内容） */
  scanFolder: (dir: string) =>
    invokeCommand<FolderFileEntry[]>('batch_scan_folder', { dir }),

  /** 读取所有文件并执行统一扫描 */
  scan: (input: BatchScanInput) =>
    invokeCommand<BatchScanResult>('batch_scan', { input }),

  /** 对所有文件应用脱敏清单 */
  execute: (input: BatchExecuteInput) =>
    invokeCommand<BatchExecuteResult>('batch_execute', { input }),

  /** 导出到文件夹（可选 ZIP） */
  export: (input: BatchExportInput) =>
    invokeCommand<string>('batch_export', { input }),

  /** 使用统一映射表还原所有文件 */
  restore: (batchSessionId: string) =>
    invokeCommand<BatchFileResult[]>('batch_restore', { batchSessionId }),

  /** 获取批量会话历史 */
  getSessions: () =>
    invokeCommand<BatchSessionListItem[]>('batch_session_get_all'),

  /** 删除批量会话 */
  deleteSession: (id: string) =>
    invokeCommand<boolean>('batch_session_delete', { id }),
};
