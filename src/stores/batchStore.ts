import { create } from 'zustand';
import { message } from 'antd';
import { batchApi } from '../services/batchApi';
import type {
  ImportedFile,
  BatchScanResult,
  BatchExecuteResult,
  BatchSessionListItem,
} from '../types/batch';
import type { ScanItem } from './sessionStore';

/** 批量扫描词条：在 ScanItem 基础上增加来源文件列表 */
export interface BatchScanItem extends ScanItem {
  sourceFiles: string[];
}

interface BatchState {
  // 导入阶段
  importedFiles: ImportedFile[];

  // 扫描阶段
  scanId: string | null;
  scanResult: BatchScanResult | null;
  scanLoading: boolean;
  scanItems: BatchScanItem[];      // 用户可编辑的识别词条（含来源文件）

  // 执行阶段
  executeResult: BatchExecuteResult | null;
  executeLoading: boolean;

  // 历史列表
  batchSessions: BatchSessionListItem[];
  sessionsLoading: boolean;

  // Actions
  setImportedFiles: (files: ImportedFile[]) => void;
  addImportedFiles: (files: ImportedFile[]) => void;
  removeImportedFile: (path: string) => void;
  clearImportedFiles: () => void;

  scan: () => Promise<BatchScanResult | null>;
  setScanItems: (items: BatchScanItem[]) => void;

  execute: () => Promise<BatchExecuteResult | null>;

  fetchSessions: () => Promise<void>;
  deleteSession: (id: string) => Promise<boolean>;

  reset: () => void;
}

const INITIAL_STATE = {
  importedFiles: [],
  scanId: null,
  scanResult: null,
  scanLoading: false,
  scanItems: [],
  executeResult: null,
  executeLoading: false,
  batchSessions: [],
  sessionsLoading: false,
};

export const useBatchStore = create<BatchState>((set, get) => ({
  ...INITIAL_STATE,

  setImportedFiles: (files) => set({ importedFiles: files }),

  addImportedFiles: (files) => {
    const { importedFiles } = get();
    // 去重（按 path）
    const existingPaths = new Set(importedFiles.map((f) => f.path));
    const newFiles = files.filter((f) => !existingPaths.has(f.path));
    set({ importedFiles: [...importedFiles, ...newFiles] });
  },

  removeImportedFile: (path) => {
    set({ importedFiles: get().importedFiles.filter((f) => f.path !== path) });
  },

  clearImportedFiles: () => set({ importedFiles: [] }),

  scan: async () => {
    const { importedFiles } = get();
    if (importedFiles.length === 0) {
      message.warning('请先选择要处理的文件');
      return null;
    }

    set({ scanLoading: true, scanId: null, scanResult: null, scanItems: [], executeResult: null });
    try {
      const result = await batchApi.scan({ filePaths: importedFiles.map((f) => f.path) });

      // 转换为可编辑的 BatchScanItem 列表（含来源文件）
      const scanItems: BatchScanItem[] = result.mergedItems.map((item, idx) => ({
        id: `batch-scan-${idx}-${Date.now()}`,
        originalValue: item.originalValue,
        entityName: item.entityName,
        strategy: item.strategy as 'random_replace' | 'empty',
        excluded: false,
        count: item.sourceFiles.length, // 出现在几个文件里
        sourceFiles: item.sourceFiles,
      }));

      set({ scanId: result.scanId, scanResult: result, scanItems, scanLoading: false });
      return result;
    } catch (err) {
      set({ scanLoading: false });
      message.error('批量扫描失败: ' + err);
      return null;
    }
  },

  setScanItems: (items) => set({ scanItems: items }),

  execute: async () => {
    const { scanId, scanItems } = get();
    if (!scanId) {
      message.warning('请先执行扫描');
      return null;
    }
    const activeItems = scanItems.filter((i) => !i.excluded);
    if (activeItems.length === 0) {
      message.warning('没有需要脱敏的项目');
      return null;
    }

    set({ executeLoading: true });
    try {
      const result = await batchApi.execute({
        scanId,
        items: activeItems.map((i) => ({
          originalValue: i.originalValue,
          entityName: i.entityName,
          strategy: i.strategy,
        })),
      });
      set({ executeResult: result, executeLoading: false });
      get().fetchSessions();
      return result;
    } catch (err) {
      set({ executeLoading: false });
      message.error('批量脱敏失败: ' + err);
      return null;
    }
  },

  fetchSessions: async () => {
    set({ sessionsLoading: true });
    try {
      const sessions = await batchApi.getSessions();
      set({ batchSessions: sessions, sessionsLoading: false });
    } catch {
      set({ sessionsLoading: false });
    }
  },

  deleteSession: async (id) => {
    try {
      const ok = await batchApi.deleteSession(id);
      if (ok) await get().fetchSessions();
      return ok;
    } catch (err) {
      message.error('删除失败: ' + err);
      return false;
    }
  },

  reset: () => set({ ...INITIAL_STATE }),
}));
