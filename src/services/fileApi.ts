import { invokeCommand } from './api';
import type { FileContent, ExportInput } from '../types/file';

export const fileApi = {
  /// 读取并解析文件内容
  /// @param path 文件绝对路径
  read: (path: string) =>
    invokeCommand<FileContent>('file_read', { path }),

  /// 导出文件
  /// @param input 导出参数（content/format/path）
  export: (input: ExportInput) =>
    invokeCommand<string>('file_export', { input }),
};
