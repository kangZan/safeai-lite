/// 文件类型枚举
export type FileType = 'docx' | 'doc' | 'xlsx' | 'xls' | 'pdf' | 'txt' | 'log' | 'unknown';

/// 文件元数据
export interface FileMetadata {
  size: number;
  pages?: number;
  sheets?: string[];
}

/// 文件内容（file_read 返回值）
export interface FileContent {
  filename: string;
  fileType: FileType;
  content: string;
  metadata?: FileMetadata;
}

/// 导出格式
export type ExportFormat = 'docx' | 'xlsx' | 'pdf' | 'txt' | 'md';

/// 导出参数
export interface ExportInput {
  content: string;
  format: ExportFormat;
  path: string;
}

/// 文件格式标签映射
export const FILE_FORMAT_LABELS: Record<ExportFormat, string> = {
  docx: 'Word (.docx)',
  xlsx: 'Excel (.xlsx)',
  pdf: 'PDF (.pdf)',
  txt: '文本 (.txt)',
  md: 'Markdown (.md)',
};

/// 文件格式图标映射（Ant Design 图标名称）
export const FILE_TYPE_ICONS: Record<string, string> = {
  docx: 'file-word',
  doc: 'file-word',
  xlsx: 'file-excel',
  xls: 'file-excel',
  pdf: 'file-pdf',
  txt: 'file-text',
  log: 'file-text',
  unknown: 'file',
};
