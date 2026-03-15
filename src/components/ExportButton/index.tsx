import { useState } from 'react';
import { Button, Select, Space, message } from 'antd';
import { DownloadOutlined } from '@ant-design/icons';
import { save } from '@tauri-apps/plugin-dialog';
import { fileApi } from '../../services/fileApi';
import type { ExportFormat } from '../../types/file';

const { Option } = Select;

/// 各格式的文件过滤器配置
const FORMAT_FILTERS: Record<ExportFormat, { name: string; extensions: string[] }> = {
  docx: { name: 'Word 文档', extensions: ['docx'] },
  xlsx: { name: 'Excel 表格', extensions: ['xlsx'] },
  pdf: { name: 'PDF 文件', extensions: ['pdf'] },
  txt: { name: '文本文件', extensions: ['txt'] },
  md: { name: 'Markdown 文件', extensions: ['md'] },
};

interface ExportButtonProps {
  content: string;
  /// 推荐导出格式（基于原文件格式）
  defaultFormat?: ExportFormat;
  /// 默认建议的文件名（不含扩展名）
  suggestedFilename?: string;
  /// 按钮显示文本
  label?: string;
  disabled?: boolean;
}

export default function ExportButton({
  content,
  defaultFormat = 'docx',
  suggestedFilename = 'export',
  label = '导出',
  disabled = false,
}: ExportButtonProps) {
  const [format, setFormat] = useState<ExportFormat>(defaultFormat);
  const [loading, setLoading] = useState(false);

  const handleExport = async () => {
    if (!content.trim()) {
      message.warning('没有可导出的内容');
      return;
    }

    setLoading(true);
    try {
      const filter = FORMAT_FILTERS[format];
      const savePath = await save({
        defaultPath: `${suggestedFilename}.${format}`,
        filters: [filter],
      });

      if (savePath) {
        await fileApi.export({
          content,
          format,
          path: savePath,
        });
        message.success(`已成功导出到: ${savePath}`);
      }
    } catch (err) {
      message.error('导出失败: ' + err);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Space>
      <Select
        value={format}
        onChange={(val) => setFormat(val as ExportFormat)}
        style={{ width: 130 }}
        size="small"
        disabled={disabled || loading}
      >
        <Option value="docx">Word (.docx)</Option>
        <Option value="xlsx">Excel (.xlsx)</Option>
        <Option value="pdf">PDF (.pdf)</Option>
        <Option value="txt">文本 (.txt)</Option>
        <Option value="md">Markdown (.md)</Option>
      </Select>
      <Button
        type="default"
        icon={<DownloadOutlined />}
        onClick={handleExport}
        loading={loading}
        disabled={disabled || !content.trim()}
        size="small"
      >
        {label}
      </Button>
    </Space>
  );
}
