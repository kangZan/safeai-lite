import { useState, useMemo } from 'react';
import {
  Modal,
  Checkbox,
  Input,
  Tag,
  Typography,
  Empty,
  Button,
  Tooltip,
} from 'antd';
import { FolderOutlined, FileOutlined, InfoCircleOutlined } from '@ant-design/icons';
import type { FolderFileEntry } from '../../types/batch';

const { Text } = Typography;

const MAX_SELECTED = 20;

const FILE_TYPE_COLORS: Record<string, string> = {
  docx: 'blue',
  doc: 'blue',
  xlsx: 'green',
  xls: 'green',
  pdf: 'red',
  txt: 'default',
  log: 'default',
};

interface FolderScanModalProps {
  open: boolean;
  files: FolderFileEntry[];
  onConfirm: (selected: FolderFileEntry[]) => void;
  onCancel: () => void;
}

export default function FolderScanModal({
  open,
  files,
  onConfirm,
  onCancel,
}: FolderScanModalProps) {
  const [search, setSearch] = useState('');
  const [checkedPaths, setCheckedPaths] = useState<Set<string>>(() => new Set());

  // 每次 files 变化时重置（弹窗重新打开）
  useMemo(() => {
    setCheckedPaths(new Set());
    setSearch('');
  }, [files]);

  const filtered = useMemo(() => {
    if (!search.trim()) return files;
    const kw = search.trim().toLowerCase();
    return files.filter(
      (f) =>
        f.filename.toLowerCase().includes(kw) ||
        f.relativePath.toLowerCase().includes(kw),
    );
  }, [files, search]);

  const checkedCount = checkedPaths.size;
  const atLimit = checkedCount >= MAX_SELECTED;

  // 全选/全不选仅针对当前过滤结果，且受上限约束
  const filteredPaths = filtered.map((f) => f.path);
  const allFilteredChecked =
    filteredPaths.length > 0 && filteredPaths.every((p) => checkedPaths.has(p));
  const someFilteredChecked = filteredPaths.some((p) => checkedPaths.has(p));

  const toggleAll = () => {
    if (allFilteredChecked) {
      // 取消当前过滤结果中的所有已选
      setCheckedPaths((prev) => {
        const next = new Set(prev);
        filteredPaths.forEach((p) => next.delete(p));
        return next;
      });
    } else {
      // 选中当前过滤结果中未选的，直到达到上限
      setCheckedPaths((prev) => {
        const next = new Set(prev);
        for (const p of filteredPaths) {
          if (next.size >= MAX_SELECTED) break;
          next.add(p);
        }
        return next;
      });
    }
  };

  const toggle = (path: string) => {
    setCheckedPaths((prev) => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else if (next.size < MAX_SELECTED) {
        next.add(path);
      }
      return next;
    });
  };

  const handleConfirm = () => {
    const selected = files.filter((f) => checkedPaths.has(f.path));
    onConfirm(selected);
  };

  return (
    <Modal
      title={
        <span>
          选择需要脱敏的文件
          <Text type="secondary" className="ml-2 text-sm font-normal">
            共找到 {files.length} 个文件
          </Text>
        </span>
      }
      open={open}
      onCancel={onCancel}
      width={640}
      footer={[
        <Button key="cancel" onClick={onCancel}>
          取消
        </Button>,
        <Tooltip
          key="confirm"
          title={checkedCount === 0 ? '请至少选择一个文件' : undefined}
        >
          <Button
            type="primary"
            disabled={checkedCount === 0}
            onClick={handleConfirm}
          >
            确认并导入（{checkedCount} 个文件）
          </Button>
        </Tooltip>,
      ]}
    >
      {/* 上限提示 */}
      <div className="mb-3 px-3 py-2 rounded bg-blue-50 border border-blue-100 flex items-center gap-2">
        <InfoCircleOutlined className="text-blue-400" />
        <Text className="text-sm text-blue-600">
          每次最多选择 {MAX_SELECTED} 个文件进行处理
        </Text>
      </div>

      {/* 搜索框 */}
      <Input
        placeholder="搜索文件名..."
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        allowClear
        className="mb-2"
      />

      {/* 全选行 */}
      <div className="flex items-center justify-between px-1 py-2 border-b mb-1">
        <Tooltip title={atLimit && !allFilteredChecked ? `已达上限 ${MAX_SELECTED} 个` : undefined}>
          <Checkbox
            checked={allFilteredChecked}
            indeterminate={!allFilteredChecked && someFilteredChecked}
            onChange={toggleAll}
          >
            <Text strong>全选当前结果</Text>
          </Checkbox>
        </Tooltip>
        <Text
          type={atLimit ? 'danger' : 'secondary'}
          className="text-sm"
        >
          已选 {checkedCount} / {MAX_SELECTED}
        </Text>
      </div>

      {/* 文件列表（全部展示，不截断） */}
      <div style={{ maxHeight: 400, overflowY: 'auto' }}>
        {filtered.length === 0 ? (
          <Empty description="没有匹配的文件" className="py-8" />
        ) : (
          filtered.map((file) => {
            const isChecked = checkedPaths.has(file.path);
            const isDisabled = atLimit && !isChecked;
            const hasSubfolder = file.relativePath.includes('/');

            return (
              <Tooltip
                key={file.path}
                title={isDisabled ? `最多选择 ${MAX_SELECTED} 个文件` : undefined}
                placement="left"
              >
                <div
                  className={`flex items-center gap-3 px-2 py-2 rounded ${
                    isDisabled
                      ? 'opacity-40 cursor-not-allowed'
                      : 'hover:bg-gray-50 cursor-pointer'
                  }`}
                  onClick={() => !isDisabled && toggle(file.path)}
                >
                  <Checkbox
                    checked={isChecked}
                    disabled={isDisabled}
                    onChange={() => toggle(file.path)}
                  />
                  {hasSubfolder ? (
                    <FolderOutlined className="text-yellow-500 flex-shrink-0" />
                  ) : (
                    <FileOutlined className="text-blue-400 flex-shrink-0" />
                  )}
                  <Text
                    className="flex-1 text-sm truncate min-w-0"
                    title={file.relativePath}
                  >
                    {hasSubfolder ? (
                      <span>
                        <Text type="secondary" className="text-xs">
                          {file.relativePath.substring(0, file.relativePath.lastIndexOf('/') + 1)}
                        </Text>
                        {file.filename}
                      </span>
                    ) : (
                      file.filename
                    )}
                  </Text>
                  <Tag
                    color={FILE_TYPE_COLORS[file.fileType] ?? 'default'}
                    className="flex-shrink-0 uppercase text-xs"
                  >
                    {file.fileType}
                  </Tag>
                </div>
              </Tooltip>
            );
          })
        )}
      </div>
    </Modal>
  );
}
