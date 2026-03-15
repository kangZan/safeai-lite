import { useState, useRef, useCallback, useEffect } from 'react';
import { message, Typography, Space, Tag, Button, Spin } from 'antd';
import {
  InboxOutlined,
  FileWordOutlined,
  FileExcelOutlined,
  FilePdfOutlined,
  FileTextOutlined,
  FileOutlined,
  CloseCircleOutlined,
} from '@ant-design/icons';
import { open } from '@tauri-apps/plugin-dialog';
import { fileApi } from '../../services/fileApi';
import type { FileContent, FileType } from '../../types/file';

const { Text } = Typography;

const ALLOWED_EXTENSIONS = ['doc', 'docx', 'xls', 'xlsx', 'pdf', 'txt', 'log'];
const MAX_FILE_SIZE = 20 * 1024 * 1024; // 20MB

/// 文件类型图标映射
const FILE_ICONS: Record<string, React.ReactNode> = {
  docx: <FileWordOutlined style={{ color: '#2b579a', fontSize: 20 }} />,
  doc: <FileWordOutlined style={{ color: '#2b579a', fontSize: 20 }} />,
  xlsx: <FileExcelOutlined style={{ color: '#217346', fontSize: 20 }} />,
  xls: <FileExcelOutlined style={{ color: '#217346', fontSize: 20 }} />,
  pdf: <FilePdfOutlined style={{ color: '#f40f02', fontSize: 20 }} />,
  txt: <FileTextOutlined style={{ color: '#555', fontSize: 20 }} />,
  log: <FileTextOutlined style={{ color: '#555', fontSize: 20 }} />,
};

interface FileUploadProps {
  onFileLoad: (content: string, fileInfo: FileContent | null) => void;
}

export default function FileUpload({ onFileLoad }: FileUploadProps) {
  const [loading, setLoading] = useState(false);
  const [loadedFile, setLoadedFile] = useState<FileContent | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  /// 校验文件扩展名
  const isAllowedExt = (filename: string): boolean => {
    const ext = filename.split('.').pop()?.toLowerCase() ?? '';
    return ALLOWED_EXTENSIONS.includes(ext);
  };

  /// 加载文件内容（接收本地绝对路径）
  const loadFile = useCallback(async (filePath: string) => {
    if (!isAllowedExt(filePath)) {
      message.error('不支持的文件格式，请选择 Word、Excel、PDF、TXT 或 LOG 文件');
      return;
    }
    setLoading(true);
    try {
      const fileContent = await fileApi.read(filePath);
      setLoadedFile(fileContent);
      onFileLoad(fileContent.content, fileContent);
      message.success(`文件 "${fileContent.filename}" 加载成功`);
    } catch (err) {
      message.error('文件解析失败: ' + String(err));
    } finally {
      setLoading(false);
    }
  }, [onFileLoad]);

  /// 监听 Tauri 文件拖入事件（用 onDragDropEvent 统一处理）
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupDrop = async () => {
      try {
        const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow');
        const webview = getCurrentWebviewWindow();
        unlisten = await webview.onDragDropEvent((event) => {
          if (event.payload.type === 'enter') {
            setDragOver(true);
          } else if (event.payload.type === 'leave') {
            setDragOver(false);
          } else if (event.payload.type === 'drop') {
            setDragOver(false);
            const paths = (event.payload as { type: 'drop'; paths: string[]; position: unknown }).paths;
            if (paths && paths.length > 0) {
              loadFile(paths[0]);
            }
          }
        });
      } catch {
        // 不在 Tauri 环境下忽略
      }
    };

    setupDrop();
    return () => { unlisten?.(); };
  }, [loadFile]);

  /// 通过系统文件对话框选择文件
  const handleOpenDialog = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: '支持的文件格式',
          extensions: ALLOWED_EXTENSIONS,
        }],
      });
      if (selected && typeof selected === 'string') {
        await loadFile(selected);
      }
    } catch (err) {
      message.error('打开文件对话框失败: ' + String(err));
    }
  };

  /// 处理 input[type=file] 选择（fallback，Tauri 下可获取 webkitRelativePath / path）
  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    // Tauri WebView 赋予了本地路径到 file 对象的扩展字段
    const filePath = (file as File & { path?: string }).path;
    if (filePath) {
      loadFile(filePath);
    } else {
      message.error('无法获取文件路径，请使用"浏览文件"按钮');
    }
    // 重置 input，允许再次选同一文件
    e.target.value = '';
  };

  /// 拖拽事件：dragover
  const handleDragOver = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setDragOver(true);
  };

  const handleDragLeave = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setDragOver(false);
  };

  /// 拖拽事件：drop —— Tauri WebView 下 DataTransferItem 可获取本地路径
  const handleDrop = (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    e.stopPropagation();
    setDragOver(false);

    const files = Array.from(e.dataTransfer.files);
    if (files.length === 0) return;

    const file = files[0];
    if (file.size > MAX_FILE_SIZE) {
      message.error('文件大小不能超过 20MB');
      return;
    }

    // Tauri 的 WebView 会在 File 对象上注入 .path
    const filePath = (file as File & { path?: string }).path;
    if (filePath) {
      loadFile(filePath);
    } else {
      message.warning('拖拽获取路径失败，请点击"浏览文件"按钮选择文件');
    }
  };

  /// 清除已加载文件
  const handleClear = () => {
    setLoadedFile(null);
    onFileLoad('', null);
  };

  /// 格式化文件大小
  const formatSize = (bytes?: number): string => {
    if (!bytes) return '';
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const getFileIcon = (fileType: FileType): React.ReactNode =>
    FILE_ICONS[fileType] || <FileOutlined style={{ fontSize: 20 }} />;

  // ── 已加载文件展示 ──
  if (loadedFile) {
    return (
      <div className="flex items-center gap-3 p-3 bg-gray-50 border border-gray-200 rounded-lg">
        {getFileIcon(loadedFile.fileType)}
        <div className="flex-1 min-w-0">
          <Text strong className="block truncate">{loadedFile.filename}</Text>
          <Space size="small">
            <Tag color="blue">{loadedFile.fileType.toUpperCase()}</Tag>
            {loadedFile.metadata?.size && (
              <Text type="secondary" className="text-xs">
                {formatSize(loadedFile.metadata.size)}
              </Text>
            )}
            {loadedFile.metadata?.sheets && (
              <Text type="secondary" className="text-xs">
                {loadedFile.metadata.sheets.length} 个工作表
              </Text>
            )}
          </Space>
        </div>
        <Button
          type="text" danger
          icon={<CloseCircleOutlined />}
          onClick={handleClear}
          size="small"
        >
          清除
        </Button>
      </div>
    );
  }

  // ── 上传区域 ──
  return (
    <Spin spinning={loading} tip="解析中...">
      {/* 隐藏的原生 file input，供点击触发 */}
      <input
        ref={inputRef}
        type="file"
        accept={ALLOWED_EXTENSIONS.map(e => `.${e}`).join(',')}
        style={{ display: 'none' }}
        onChange={handleInputChange}
      />
      <div
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
        onClick={handleOpenDialog}
        style={{
          border: `2px dashed ${dragOver ? '#1677ff' : '#d9d9d9'}`,
          borderRadius: 8,
          padding: '20px 16px',
          textAlign: 'center',
          cursor: 'pointer',
          background: dragOver ? '#e6f4ff' : '#fafafa',
          transition: 'all 0.2s',
          userSelect: 'none',
        }}
      >
        <InboxOutlined style={{ fontSize: 32, color: dragOver ? '#1677ff' : '#40a9ff' }} />
        <p style={{ margin: '8px 0 4px', fontSize: 14, color: '#333' }}>
          点击或拖拽文件到此区域
        </p>
        <p style={{ margin: 0, fontSize: 12, color: '#999' }}>
          支持 Word、Excel、PDF、TXT、LOG 格式，最大 20MB
        </p>
      </div>
    </Spin>
  );
}
