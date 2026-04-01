import type { DesensitizeItem } from './session';

// ── 文件夹扫描 ────────────────────────────────────────────────

export interface FolderFileEntry {
  path: string;
  filename: string;
  relativePath: string;
  fileType: string;
}

// ── 批量扫描 ──────────────────────────────────────────────────

export interface BatchScanInput {
  filePaths: string[];
}

export interface BatchFileScanStatus {
  path: string;
  filename: string;
  relativePath: string;
  fileType: string;
  status: 'success' | 'failed';
  errorMsg?: string;
  charCount: number;
}

/** 识别词条，含来源文件列表 */
export interface BatchMergedItem {
  originalValue: string;
  entityName: string;
  strategy: string;
  /** 该词出现在哪些文件中（filename 列表，已去重） */
  sourceFiles: string[];
}

export interface BatchScanResult {
  /** 后端缓存 key，execute 时凭此取回文件内容 */
  scanId: string;
  files: BatchFileScanStatus[];
  mergedItems: BatchMergedItem[];
}

// ── 批量执行 ──────────────────────────────────────────────────

export interface BatchExecuteInput {
  scanId: string;
  items: DesensitizeItem[];
}

export interface BatchFileResult {
  filename: string;
  relativePath: string;
  fileType: string;
  status: string;
  errorMsg?: string; // status==='restored' 时此字段携带还原后内容
}

export interface BatchExecuteResult {
  batchSessionId: string;
  fileCount: number;
  successCount: number;
  mappingCount: number;
  files: BatchFileResult[];
  createdAt: string;
}

// ── 批量导出 ──────────────────────────────────────────────────

export interface BatchExportInput {
  batchSessionId: string;
  outputDir: string;
  zip: boolean;
}

// ── 历史列表 ──────────────────────────────────────────────────

export interface BatchSessionListItem {
  id: string;
  name: string;
  fileCount: number;
  successCount: number;
  mappingCount: number;
  createdAt: string;
}

// ── 前端扩展：可编辑的批量导入文件项 ──────────────────────────

export interface ImportedFile {
  /** 绝对路径 */
  path: string;
  filename: string;
  /** 相对于所选文件夹的路径；直接多选时退化为文件名 */
  relativePath: string;
  fileType: string;
}
