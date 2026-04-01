import { useState, useEffect, useMemo, useCallback } from 'react';
import {
  Button,
  Card,
  Typography,
  Space,
  Tag,
  Table,
  Select,
  Checkbox,
  Tooltip,
  Badge,
  message,
  Divider,
  Alert,
  Layout,
  List,
  Popconfirm,
  Empty,
  Modal,
  Form,
  Input,
  Switch,
} from 'antd';
import {
  FolderOpenOutlined,
  UploadOutlined,
  ScanOutlined,
  ThunderboltOutlined,
  ExportOutlined,
  DeleteOutlined,
  ReloadOutlined,
  FileOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  HistoryOutlined,
  PlusOutlined,
  EyeOutlined,
  EyeInvisibleOutlined,
} from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import { open } from '@tauri-apps/plugin-dialog';
import { batchApi } from '../../services/batchApi';
import { useBatchStore } from '../../stores/batchStore';
import type { ImportedFile, BatchFileScanStatus, BatchFileResult, FolderFileEntry } from '../../types/batch';
import type { BatchScanItem } from '../../stores/batchStore';
import type { ScanItem } from '../../stores/sessionStore';
import FolderScanModal from '../../components/FolderScanModal';
import { entityApi } from '../../services/entityApi';
import type { Entity } from '../../types/entity';

const { Title, Text } = Typography;
const { Content, Sider } = Layout;

const FILE_TYPE_COLORS: Record<string, string> = {
  docx: 'blue', doc: 'blue',
  xlsx: 'green', xls: 'green',
  pdf: 'red', txt: 'default', log: 'default',
};

// ── 导入文件区 ────────────────────────────────────────────────

function ImportSection() {
  const { importedFiles, addImportedFiles, removeImportedFile, clearImportedFiles } = useBatchStore();
  const [folderScanOpen, setFolderScanOpen] = useState(false);
  const [folderFiles, setFolderFiles] = useState<FolderFileEntry[]>([]);

  const handleMultiFileSelect = async () => {
    const paths = await open({
      multiple: true,
      filters: [{ name: '支持的文件', extensions: ['doc', 'docx', 'xls', 'xlsx', 'pdf', 'txt', 'log'] }],
    });
    if (!paths) return;
    const pathArray = Array.isArray(paths) ? paths : [paths];
    const files: ImportedFile[] = pathArray.map((p) => {
      const parts = p.replace(/\\/g, '/').split('/');
      const filename = parts[parts.length - 1];
      const ext = filename.split('.').pop()?.toLowerCase() ?? '';
      return { path: p, filename, relativePath: filename, fileType: ext };
    });
    addImportedFiles(files);
  };

  const handleFolderSelect = async () => {
    const dir = await open({ directory: true });
    if (!dir || Array.isArray(dir)) return;
    try {
      const entries = await batchApi.scanFolder(dir);
      if (entries.length === 0) {
        message.warning('所选文件夹中没有可处理的文件格式');
        return;
      }
      setFolderFiles(entries);
      setFolderScanOpen(true);
    } catch (err) {
      message.error('扫描文件夹失败: ' + err);
    }
  };

  const handleFolderConfirm = (selected: FolderFileEntry[]) => {
    const files: ImportedFile[] = selected.map((e) => ({
      path: e.path,
      filename: e.filename,
      relativePath: e.relativePath,
      fileType: e.fileType,
    }));
    addImportedFiles(files);
    setFolderScanOpen(false);
  };

  return (
    <>
      <Card
        size="small"
        title={<span><UploadOutlined className="mr-1" />选择文件</span>}
        extra={
          importedFiles.length > 0 && (
            <Button size="small" danger onClick={clearImportedFiles}>
              清空列表
            </Button>
          )
        }
      >
        <Space className="mb-3">
          <Button icon={<UploadOutlined />} onClick={handleMultiFileSelect}>
            选择多个文件
          </Button>
          <Button icon={<FolderOpenOutlined />} onClick={handleFolderSelect}>
            选择文件夹
          </Button>
        </Space>

        {importedFiles.length === 0 ? (
          <div className="text-center py-4 text-gray-400 text-sm">
            请选择文件或文件夹，批量导入需要脱敏的文档
          </div>
        ) : (
          <div style={{ maxHeight: 200, overflowY: 'auto' }}>
            {importedFiles.map((f) => (
              <div
                key={f.path}
                className="flex items-center gap-2 py-1 px-2 rounded hover:bg-gray-50"
              >
                <FileOutlined className="text-gray-400 flex-shrink-0" />
                <Text className="flex-1 text-sm truncate" title={f.relativePath}>
                  {f.relativePath !== f.filename ? (
                    <span>
                      <Text type="secondary" className="text-xs">{f.relativePath.substring(0, f.relativePath.lastIndexOf('/') + 1)}</Text>
                      {f.filename}
                    </span>
                  ) : f.filename}
                </Text>
                <Tag color={FILE_TYPE_COLORS[f.fileType] ?? 'default'} className="flex-shrink-0">
                  {f.fileType}
                </Tag>
                <Button
                  type="text"
                  size="small"
                  icon={<DeleteOutlined />}
                  danger
                  onClick={() => removeImportedFile(f.path)}
                />
              </div>
            ))}
          </div>
        )}
      </Card>

      <FolderScanModal
        open={folderScanOpen}
        files={folderFiles}
        onConfirm={handleFolderConfirm}
        onCancel={() => setFolderScanOpen(false)}
      />
    </>
  );
}

// ── 新增词条弹窗（批量版）────────────────────────────────────

function AddItemsModal({
  open: modalOpen,
  onAdd,
  onClose,
}: {
  open: boolean;
  onAdd: (items: Omit<ScanItem, 'id' | 'excluded' | 'count'>[]) => void;
  onClose: () => void;
}) {
  const [tab, setTab] = useState('entity');
  const [entities, setEntities] = useState<Entity[]>([]);
  const [loadingEntities, setLoadingEntities] = useState(false);
  const [selectedEntityId, setSelectedEntityId] = useState<string | null>(null);
  const [selectedSynonyms, setSelectedSynonyms] = useState<string[]>([]);
  const [newWordInput, setNewWordInput] = useState('');
  const [pendingNewWords, setPendingNewWords] = useState<string[]>([]);
  const [saveToEntity, setSaveToEntity] = useState(true);
  const [showSynonyms, setShowSynonyms] = useState(true);
  const [manualForm] = Form.useForm<{ originalValue: string; entityName: string; strategy: 'random_replace' | 'empty' }>();

  useMemo(() => {
    if (modalOpen) {
      setLoadingEntities(true);
      entityApi.getAll()
        .then((list) => setEntities(list))
        .catch(() => message.error('加载实体列表失败'))
        .finally(() => setLoadingEntities(false));
      setSelectedEntityId(null);
      setSelectedSynonyms([]);
      setNewWordInput('');
      setPendingNewWords([]);
      setSaveToEntity(true);
      manualForm.resetFields();
    }
  }, [modalOpen, manualForm]);

  const selectedEntity = entities.find((e) => e.id === selectedEntityId);

  const handleAddNewWord = () => {
    const word = newWordInput.trim();
    if (!word) return;
    if (pendingNewWords.includes(word) || selectedEntity?.synonyms.includes(word)) {
      message.warning('该匹配词已存在');
      return;
    }
    setPendingNewWords((prev) => [...prev, word]);
    setSelectedSynonyms((prev) => [...prev, word]);
    setNewWordInput('');
  };

  const handleConfirm = async () => {
    if (tab === 'entity') {
      if (!selectedEntity || selectedSynonyms.length === 0) {
        message.warning('请先选择实体并勾选匹配词');
        return;
      }
      if (saveToEntity && pendingNewWords.length > 0) {
        try {
          const merged = [...selectedEntity.synonyms, ...pendingNewWords];
          await entityApi.updateSynonyms(selectedEntity.id, merged);
          message.success(`已将 ${pendingNewWords.length} 个新词保存到「${selectedEntity.name}」`);
        } catch (err) {
          message.warning(`保存到实体失败：${String(err)}`);
        }
      }
      onAdd(
        selectedSynonyms.map((v) => ({
          originalValue: v,
          entityName: selectedEntity.name,
          strategy: selectedEntity.strategy,
        }))
      );
      onClose();
    } else {
      manualForm.validateFields().then((vals) => {
        onAdd([{
          originalValue: vals.originalValue.trim(),
          entityName: vals.entityName.trim(),
          strategy: vals.strategy,
        }]);
        onClose();
      });
    }
  };

  const tabItems = [
    { key: 'entity', label: '从实体选择' },
    { key: 'manual', label: '手动输入词条' },
  ];

  return (
    <Modal
      title="新增词条"
      open={modalOpen}
      onOk={handleConfirm}
      onCancel={onClose}
      okText="添加到清单"
      cancelText="取消"
      width={520}
      destroyOnClose
    >
      <div className="flex gap-4 mb-3 border-b pb-2">
        {tabItems.map((t) => (
          <button
            key={t.key}
            className={`text-sm px-1 pb-1 border-b-2 transition-colors ${tab === t.key ? 'border-blue-500 text-blue-600' : 'border-transparent text-gray-500 hover:text-gray-700'}`}
            onClick={() => setTab(t.key)}
          >
            {t.label}
          </button>
        ))}
      </div>

      {tab === 'entity' && (
        <div>
          <div className="mb-3">
            <Typography.Text type="secondary" className="text-xs">
              选择实体后可勾选已有匹配词，也可直接输入新词——内置实体同样支持新增匹配词
            </Typography.Text>
          </div>
          <div className="mb-3">
            <Select
              style={{ width: '100%' }}
              placeholder="选择实体（内置或自定义）"
              loading={loadingEntities}
              onChange={(id) => {
                setSelectedEntityId(id);
                setSelectedSynonyms([]);
                setPendingNewWords([]);
                setNewWordInput('');
              }}
              options={entities.map((e) => ({
                value: e.id,
                label: (
                  <Space size={4}>
                    {e.name}
                    <Tag style={{ fontSize: 11 }} color={e.entityType === 'builtin' ? 'blue' : 'green'}>
                      {e.entityType === 'builtin' ? '内置' : '自定义'}
                    </Tag>
                  </Space>
                ),
              }))}
            />
          </div>
          {selectedEntity && (
            <div>
              {(selectedEntity.synonyms.length > 0 || pendingNewWords.length > 0) ? (
                <List
                  size="small"
                  bordered
                  style={{ marginBottom: 12 }}
                  header={
                    <div className="flex items-center justify-between">
                      <Typography.Text className="text-xs text-gray-500">匹配词列表（勾选要加入清单的）</Typography.Text>
                      <Switch
                        size="small"
                        checked={showSynonyms}
                        onChange={setShowSynonyms}
                        checkedChildren={<EyeOutlined />}
                        unCheckedChildren={<EyeInvisibleOutlined />}
                      />
                    </div>
                  }
                  dataSource={[...selectedEntity.synonyms, ...pendingNewWords]}
                  renderItem={(syn) => {
                    const isNew = pendingNewWords.includes(syn);
                    return (
                      <List.Item>
                        <Checkbox
                          checked={selectedSynonyms.includes(syn)}
                          onChange={(e) => {
                            if (e.target.checked) {
                              setSelectedSynonyms((prev) => [...prev, syn]);
                            } else {
                              setSelectedSynonyms((prev) => prev.filter((s) => s !== syn));
                            }
                          }}
                        >
                          <Space size={4}>
                            {showSynonyms ? syn : '******'}
                            {isNew && <Tag color="orange" style={{ fontSize: 11 }}>新增</Tag>}
                          </Space>
                        </Checkbox>
                      </List.Item>
                    );
                  }}
                />
              ) : (
                <Typography.Text type="secondary" className="text-xs" style={{ display: 'block', marginBottom: 12 }}>
                  该实体暂无匹配词，可在下方输入新增
                </Typography.Text>
              )}
              <Space.Compact style={{ width: '100%', marginBottom: 8 }}>
                <Input
                  placeholder="输入新匹配词，回车或点击「+」添加"
                  value={newWordInput}
                  onChange={(e) => setNewWordInput(e.target.value)}
                  onPressEnter={handleAddNewWord}
                />
                <Button type="primary" onClick={handleAddNewWord}>+</Button>
              </Space.Compact>
              {pendingNewWords.length > 0 && (
                <Checkbox
                  checked={saveToEntity}
                  onChange={(e) => setSaveToEntity(e.target.checked)}
                >
                  <Typography.Text className="text-xs">
                    同时保存到「{selectedEntity.name}」实体配置，下次自动识别
                  </Typography.Text>
                </Checkbox>
              )}
            </div>
          )}
        </div>
      )}

      {tab === 'manual' && (
        <Form form={manualForm} layout="vertical">
          <Form.Item
            name="originalValue"
            label="原文词语"
            rules={[{ required: true, message: '请输入需要脱敏的原文词语' }]}
          >
            <Input placeholder="如：张三" />
          </Form.Item>
          <Form.Item
            name="entityName"
            label="实体名称（标注用途）"
            rules={[{ required: true, message: '请输入实体名称' }]}
          >
            <Input placeholder="如：姓名" />
          </Form.Item>
          <Form.Item
            name="strategy"
            label="替换策略"
            initialValue="random_replace"
            rules={[{ required: true }]}
          >
            <Select
              options={[
                { value: 'random_replace', label: '随机数据替换（可还原）' },
                { value: 'empty', label: '置空（不可还原）' },
              ]}
            />
          </Form.Item>
        </Form>
      )}
    </Modal>
  );
}

// ── 扫描结果编辑表 ────────────────────────────────────────────

function ScanItemsTable() {
  const { scanItems, setScanItems } = useBatchStore();

  const updateItem = (id: string, patch: Partial<BatchScanItem>) => {
    setScanItems(scanItems.map((i) => (i.id === id ? { ...i, ...patch } : i)));
  };

  const columns: ColumnsType<BatchScanItem> = [
    {
      title: '排除',
      dataIndex: 'excluded',
      width: 55,
      render: (_, row) => (
        <Checkbox
          checked={row.excluded}
          onChange={(e) => updateItem(row.id, { excluded: e.target.checked })}
        />
      ),
    },
    {
      title: '识别值',
      dataIndex: 'originalValue',
      render: (v, row) => (
        <Text delete={row.excluded} className="text-sm">{v}</Text>
      ),
    },
    {
      title: '类型',
      dataIndex: 'entityName',
      width: 120,
      render: (v) => <Tag color="processing">{v}</Tag>,
    },
    {
      title: '来源文件',
      dataIndex: 'sourceFiles',
      render: (files: string[]) => (
        <div className="flex flex-wrap gap-1">
          {files.map((f) => (
            <Tooltip key={f} title={f}>
              <Tag className="max-w-32 truncate text-xs" style={{ cursor: 'default' }}>
                {f}
              </Tag>
            </Tooltip>
          ))}
        </div>
      ),
    },
    {
      title: '策略',
      dataIndex: 'strategy',
      width: 120,
      render: (v, row) => (
        <Select
          size="small"
          value={v}
          disabled={row.excluded}
          onChange={(val) => updateItem(row.id, { strategy: val })}
          options={[
            { label: '随机替换', value: 'random_replace' },
            { label: '置空', value: 'empty' },
          ]}
          style={{ width: '100%' }}
        />
      ),
    },
  ];

  return (
    <Table
      size="small"
      dataSource={scanItems}
      rowKey="id"
      columns={columns}
      pagination={{ pageSize: 10, size: 'small', showSizeChanger: false }}
      locale={{ emptyText: '暂无识别结果' }}
      scroll={{ y: 300 }}
    />
  );
}

// ── 文件状态列表（扫描 / 执行结果）────────────────────────────

function FileStatusList({
  files,
}: {
  files: Array<BatchFileScanStatus | BatchFileResult>;
}) {
  if (files.length === 0) return null;
  return (
    <div style={{ maxHeight: 200, overflowY: 'auto' }}>
      {files.map((f, idx) => {
        const isOk = f.status === 'success' || f.status === 'restored';
        return (
          <div key={idx} className="flex items-center gap-2 py-1 px-2 rounded hover:bg-gray-50">
            {isOk ? (
              <CheckCircleOutlined className="text-green-500 flex-shrink-0" />
            ) : (
              <CloseCircleOutlined className="text-red-500 flex-shrink-0" />
            )}
            <Text className="flex-1 text-sm truncate" title={f.relativePath}>
              {f.relativePath}
            </Text>
            <Tag color={FILE_TYPE_COLORS[f.fileType] ?? 'default'} className="flex-shrink-0">
              {f.fileType}
            </Tag>
            {!isOk && f.errorMsg && (
              <Tooltip title={f.errorMsg}>
                <Text type="danger" className="text-xs truncate max-w-32">{f.errorMsg}</Text>
              </Tooltip>
            )}
          </div>
        );
      })}
    </div>
  );
}

// ── 批量导出面板 ──────────────────────────────────────────────

function ExportPanel({ batchSessionId }: { batchSessionId: string }) {
  const [exporting, setExporting] = useState(false);

  const handleExport = async () => {
    const dir = await open({ directory: true });
    if (!dir || Array.isArray(dir)) return;
    setExporting(true);
    try {
      const outPath = await batchApi.export({
        batchSessionId,
        outputDir: dir as string,
        zip: false,
      });
      message.success(`导出完成：${outPath}`);
    } catch (err) {
      message.error('导出失败: ' + err);
    } finally {
      setExporting(false);
    }
  };

  return (
    <Button
      type="primary"
      icon={<ExportOutlined />}
      loading={exporting}
      onClick={handleExport}
    >
      选择导出文件夹
    </Button>
  );
}

// ── 历史侧边栏 ────────────────────────────────────────────────

function BatchSessionSider() {
  const { batchSessions, sessionsLoading, fetchSessions, deleteSession } = useBatchStore();

  useEffect(() => {
    fetchSessions();
  }, [fetchSessions]);

  return (
    <Sider
      width={220}
      style={{
        background: '#fafafa',
        borderLeft: '1px solid #f0f0f0',
        overflowY: 'auto',
        padding: '12px 8px',
      }}
    >
      <div className="flex items-center justify-between mb-2 px-1">
        <Text strong className="text-sm">
          <HistoryOutlined className="mr-1" />
          批量历史
        </Text>
        <Button
          type="text"
          size="small"
          icon={<ReloadOutlined />}
          onClick={fetchSessions}
          loading={sessionsLoading}
        />
      </div>

      {batchSessions.length === 0 ? (
        <Empty description="暂无历史" image={Empty.PRESENTED_IMAGE_SIMPLE} className="mt-4" />
      ) : (
        <List
          size="small"
          dataSource={batchSessions}
          renderItem={(item) => (
            <List.Item
              style={{ padding: '6px 4px', cursor: 'default' }}
              actions={[
                <Popconfirm
                  key="del"
                  title="删除此批量会话？"
                  onConfirm={() => deleteSession(item.id)}
                  okText="删除"
                  cancelText="取消"
                  okButtonProps={{ danger: true }}
                >
                  <Button type="text" size="small" icon={<DeleteOutlined />} danger />
                </Popconfirm>,
              ]}
            >
              <div className="w-full min-w-0">
                <Text className="text-xs block truncate" title={item.name}>
                  {item.name}
                </Text>
                <Text type="secondary" className="text-xs">
                  {item.successCount}/{item.fileCount} 文件 · {item.mappingCount} 词条
                </Text>
              </div>
            </List.Item>
          )}
        />
      )}
    </Sider>
  );
}

// ── 主页面 ────────────────────────────────────────────────────

export default function BatchDesensitize() {
  const {
    importedFiles,
    scanResult,
    scanLoading,
    scanItems,
    setScanItems,
    executeResult,
    executeLoading,
    scan,
    execute,
  } = useBatchStore();

  const [addModalOpen, setAddModalOpen] = useState(false);

  const successFiles = scanResult?.files.filter((f) => f.status === 'success') ?? [];
  const failedFiles = scanResult?.files.filter((f) => f.status === 'failed') ?? [];

  const activeItemCount = scanItems.filter((i) => !i.excluded).length;

  const handleAddItems = useCallback((newItems: Omit<ScanItem, 'id' | 'excluded' | 'count'>[]) => {
    const toAdd: BatchScanItem[] = newItems.map((item, idx) => ({
      ...item,
      id: `manual-${Date.now()}-${idx}`,
      excluded: false,
      count: 1,
      sourceFiles: ['手动添加'],
    }));
    setScanItems([...scanItems, ...toAdd]);
    message.success(`已添加 ${toAdd.length} 个词条`);
  }, [scanItems, setScanItems]);

  return (
    <Layout style={{ height: '100%', background: '#fff' }}>
      <Content style={{ padding: '16px', overflowY: 'auto' }}>
        <div className="flex items-center gap-3 mb-4">
          <Title level={4} style={{ margin: 0 }}>批量脱敏</Title>
          <Tag color="blue">v0.2.0</Tag>
          <Text type="secondary" className="text-sm">
            同时处理多个文件，共享同一套脱敏映射表
          </Text>
        </div>

        {/* Stage 0: 导入文件 */}
        <ImportSection />

        <Divider className="my-3" />

        {/* Stage 1: 扫描 */}
        <div className="flex items-center gap-3 mb-3">
          <Button
            type="primary"
            icon={<ScanOutlined />}
            loading={scanLoading}
            disabled={importedFiles.length === 0}
            onClick={scan}
          >
            识别敏感词（{importedFiles.length} 个文件）
          </Button>
          {scanResult && (
            <Text type="secondary" className="text-sm">
              共识别 {scanResult.mergedItems.length} 个词条 ·
              成功解析 {successFiles.length} 个文件
              {failedFiles.length > 0 && (
                <Text type="danger"> · {failedFiles.length} 个失败</Text>
              )}
            </Text>
          )}
        </div>

        {/* 扫描失败文件提示 */}
        {failedFiles.length > 0 && (
          <Alert
            type="warning"
            showIcon
            className="mb-3"
            message={`${failedFiles.length} 个文件解析失败，将跳过处理`}
            description={
              <FileStatusList files={failedFiles} />
            }
          />
        )}

        {/* 识别清单 */}
        {scanItems.length > 0 && (
          <Card
            size="small"
            title={
              <span>
                识别清单
                <Badge count={activeItemCount} className="ml-2" />
                <Text type="secondary" className="text-xs ml-2">（可编辑排除项和替换策略）</Text>
              </span>
            }
            extra={
              <Tooltip title="新增本次未识别但需要脱敏的词条">
                <Button
                  icon={<PlusOutlined />}
                  size="small"
                  onClick={() => setAddModalOpen(true)}
                >
                  新增词条
                </Button>
              </Tooltip>
            }
            className="mb-3"
          >
            <ScanItemsTable />
          </Card>
        )}

        {/* Stage 2: 执行脱敏 */}
        {scanItems.length > 0 && (
          <>
            <div className="flex items-center gap-3 mb-3">
              <Button
                type="primary"
                danger
                icon={<ThunderboltOutlined />}
                loading={executeLoading}
                disabled={activeItemCount === 0}
                onClick={execute}
              >
                执行脱敏（{activeItemCount} 个词条）
              </Button>
              {executeResult && (
                <Text type="secondary" className="text-sm">
                  脱敏完成：{executeResult.successCount}/{executeResult.fileCount} 个文件 ·
                  {executeResult.mappingCount} 个映射
                </Text>
              )}
            </div>
          </>
        )}

        {/* Stage 3: 执行结果 + 导出 */}
        {executeResult && (
          <Card
            size="small"
            title={
              <span>
                <CheckCircleOutlined className="text-green-500 mr-1" />
                脱敏完成
              </span>
            }
            className="mb-3"
          >
            <FileStatusList files={executeResult.files} />
            <Divider className="my-3" />
            <div>
              <Text strong className="block mb-2">导出脱敏结果</Text>
              <ExportPanel batchSessionId={executeResult.batchSessionId} />
            </div>
          </Card>
        )}
      </Content>

      <BatchSessionSider />

      <AddItemsModal
        open={addModalOpen}
        onAdd={handleAddItems}
        onClose={() => setAddModalOpen(false)}
      />
    </Layout>
  );
}
