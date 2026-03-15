import { useState, useMemo, useCallback, useEffect } from 'react';
import {
  Alert,
  Card,
  Input,
  Button,
  Typography,
  message,
  Divider,
  Space,
  Tabs,
  Tooltip,
  Layout,
  Table,
  Tag,
  Switch,
  Checkbox,
  Modal,
  Select,
  Form,
  List,
  Badge,
} from 'antd';
import {
  CopyOutlined,
  ThunderboltOutlined,
  RollbackOutlined,
  FileMarkdownOutlined,
  SafetyOutlined,
  EyeOutlined,
  EyeInvisibleOutlined,
  ScanOutlined,
  PlusOutlined,
  ReloadOutlined,
} from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import { save } from '@tauri-apps/plugin-dialog';
import { fileApi } from '../../services/fileApi';
import { desensitizeApi } from '../../services/desensitizeApi';
import type { NerStatus } from '../../services/desensitizeApi';
import { useSessionStore } from '../../stores/sessionStore';
import type { ScanItem } from '../../stores/sessionStore';
import { entityApi } from '../../services/entityApi';
import type { Entity } from '../../types/entity';
import FileUpload from '../../components/FileUpload';
import ExportButton from '../../components/ExportButton';
import SessionList from '../../components/SessionList';
import type { FileContent } from '../../types/file';
import type { ExportFormat } from '../../types/file';

const { Title, Text } = Typography;
const { TextArea } = Input;
const { Sider, Content } = Layout;

/** 新增词条弹窗：Tab1 从实体选择/新增匹配词，Tab2 手动输入原文 */
function AddItemsModal({
  open,
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
  // 本次新输入的匹配词（未保存到实体）
  const [newWordInput, setNewWordInput] = useState('');
  const [pendingNewWords, setPendingNewWords] = useState<string[]>([]);
  const [saveToEntity, setSaveToEntity] = useState(true);
  const [manualForm] = Form.useForm<{ originalValue: string; entityName: string; strategy: 'random_replace' | 'empty' }>();

  // 打开时加载全部实体（含内置）
  useMemo(() => {
    if (open) {
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
  }, [open, manualForm]);

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
      // 若有新增词且勾选了保存，写入实体配置
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

  return (
    <Modal
      title="新增词条"
      open={open}
      onOk={handleConfirm}
      onCancel={onClose}
      okText="添加到清单"
      cancelText="取消"
      width={520}
      destroyOnClose
    >
      <Tabs
        activeKey={tab}
        onChange={setTab}
        items={[
          { key: 'entity', label: '从实体选择' },
          { key: 'manual', label: '手动输入词条' },
        ]}
      />

      {tab === 'entity' && (
        <div>
          <div className="mb-3">
            <Text type="secondary" className="text-xs">
              选择实体后可勾选已有匹配词，也可直接输入新词——内置实体同样支持新增匹配词
            </Text>
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
              {/* 已有匹配词列表 */}
              {(selectedEntity.synonyms.length > 0 || pendingNewWords.length > 0) ? (
                <List
                  size="small"
                  bordered
                  style={{ marginBottom: 12 }}
                  header={<Text className="text-xs text-gray-500">匹配词列表（勾选要加入清单的）</Text>}
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
                            {syn}
                            {isNew && <Tag color="orange" style={{ fontSize: 11 }}>新增</Tag>}
                          </Space>
                        </Checkbox>
                      </List.Item>
                    );
                  }}
                />
              ) : (
                <Text type="secondary" className="text-xs" style={{ display: 'block', marginBottom: 12 }}>
                  该实体暂无匹配词，可在下方输入新增
                </Text>
              )}
              {/* 新增匹配词输入框 */}
              <Space.Compact style={{ width: '100%', marginBottom: 8 }}>
                <Input
                  placeholder="输入新匹配词，回车或点击「+」添加"
                  value={newWordInput}
                  onChange={(e) => setNewWordInput(e.target.value)}
                  onPressEnter={handleAddNewWord}
                />
                <Button type="primary" onClick={handleAddNewWord}>+</Button>
              </Space.Compact>
              {/* 是否保存到实体配置 */}
              {pendingNewWords.length > 0 && (
                <Checkbox
                  checked={saveToEntity}
                  onChange={(e) => setSaveToEntity(e.target.checked)}
                >
                  <Text className="text-xs">
                    同时保存到「{selectedEntity.name}」实体配置，下次自动识别
                  </Text>
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

export default function Desensitize() {
  const [inputContent, setInputContent] = useState('');
  const [restoreContent, setRestoreContent] = useState('');
  const [loadedFile, setLoadedFile] = useState<FileContent | null>(null);
  const [exportMdLoading, setExportMdLoading] = useState(false);
  // 识别清单：是否显示原文
  const [showOriginalInList, setShowOriginalInList] = useState(false);
  // 新增词条弹窗
  const [addModalOpen, setAddModalOpen] = useState(false);
  // NER 模型状态
  const [nerStatus, setNerStatus] = useState<NerStatus | null>(null);

  // 启动时查询 NER 状态，加载中时轮询直到就绪或失败
  useEffect(() => {
    let cancelled = false;
    const check = async () => {
      try {
        const status = await desensitizeApi.getNerStatus();
        if (cancelled) return;
        setNerStatus(status);
        if (status.loading) {
          setTimeout(check, 1000);
        }
      } catch {
        // 忽略查询错误
      }
    };
    check();
    return () => { cancelled = true; };
  }, []);

  const {
    scanItems,
    scanLoading,
    desensitizedContent,
    restoredContent,
    currentSessionId,
    loading,
    scan,
    setScanItems,
    desensitize,
    restore,
    loadSession,
    clear,
  } = useSessionStore();

  // 是否已完成脱敏（脱敏结果是否已修改过，需要重新脱敏）
  const hasResult = !!desensitizedContent;

  // ===== 扫描 =====
  const handleScan = async () => {
    if (!inputContent.trim()) {
      message.warning('请输入需要识别的内容');
      return;
    }
    const result = await scan(inputContent);
    // 扫描完毕后刷新 NER 状态
    try {
      const status = await desensitizeApi.getNerStatus();
      setNerStatus(status);
    } catch { /* ignore */ }

    if (result && result.items.length === 0) {
      if (nerStatus && !nerStatus.ready) {
        if (nerStatus.error) {
          message.warning(`未识别到敏感词。NER 模型加载失败：${nerStatus.error}`);
        } else {
          message.warning('未识别到敏感词。NER 模型尚未就绪，姓名/公司/地址识别可能不完整，请稍后重试。');
        }
      } else {
        message.info('未识别到敏感词，请检查自定义实体配置');
      }
    }
  };

  // ===== 执行脱敏 =====
  const handleDesensitize = async () => {
    if (!inputContent.trim()) {
      message.warning('请先输入内容并识别敏感词');
      return;
    }
    if (scanItems.length === 0) {
      message.warning('清单为空，请先点击"识别敏感词"');
      return;
    }
    await desensitize(inputContent, scanItems);
  };

  // ===== 清单编辑 =====
  const toggleExclude = useCallback((id: string) => {
    setScanItems(
      scanItems.map((item) =>
        item.id === id ? { ...item, excluded: !item.excluded } : item
      )
    );
  }, [scanItems, setScanItems]);

  const removeItem = useCallback((id: string) => {
    setScanItems(scanItems.filter((item) => item.id !== id));
  }, [scanItems, setScanItems]);

  const invertSelection = useCallback(() => {
    setScanItems(scanItems.map((item) => ({ ...item, excluded: !item.excluded })));
  }, [scanItems, setScanItems]);

  const handleAddItems = useCallback((newItems: Omit<ScanItem, 'id' | 'excluded' | 'count'>[]) => {
    const toAdd: ScanItem[] = newItems.map((item, idx) => {
      let count = 0;
      let pos = 0;
      while ((pos = inputContent.indexOf(item.originalValue, pos)) !== -1) {
        count++;
        pos += item.originalValue.length;
      }
      return {
        ...item,
        id: `manual-${Date.now()}-${idx}`,
        excluded: false,
        count: Math.max(count, 1),
      };
    });
    setScanItems([...scanItems, ...toAdd]);
    message.success(`已添加 ${toAdd.length} 个词条`);
  }, [scanItems, setScanItems, inputContent]);

  // ===== 还原 =====
  const handleRestore = async () => {
    if (!restoreContent.trim()) {
      message.warning('请输入需要还原的内容');
      return;
    }
    if (!currentSessionId) {
      message.warning('请先执行脱敏操作');
      return;
    }
    await restore(restoreContent);
  };

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    message.success('已复制到剪贴板');
  };

  /// 文件加载后回调：将解析内容填入输入框
  const handleFileLoad = (content: string, fileInfo: FileContent | null) => {
    setInputContent(content);
    setLoadedFile(fileInfo);
  };

  /// 新增会话：清空当前状态，准备全新脱敏
  const handleNewSession = () => {
    clear();
    setInputContent('');
    setLoadedFile(null);
    setRestoreContent('');
  };

  /// 点击会话列表项：加载历史会话数据
  const handleSessionSelect = async (sessionId: string) => {
    const detail = await loadSession(sessionId);
    if (detail) {
      setInputContent(detail.originalContent);
      setRestoreContent('');
    }
  };

  /// 导出脱敏结果为 Markdown 文件
  const handleExportMd = async () => {
    if (!desensitizedContent) {
      message.warning('没有可导出的脱敏结果');
      return;
    }

    setExportMdLoading(true);
    try {
      const baseName = loadedFile
        ? loadedFile.filename.replace(/\.[^.]+$/, '')
        : `脱敏结果_${new Date().toISOString().slice(0, 10)}`;

      const savePath = await save({
        defaultPath: `${baseName}_脱敏.md`,
        filters: [{ name: 'Markdown 文件', extensions: ['md'] }],
      });

      if (savePath) {
        await fileApi.export({
          content: desensitizedContent,
          format: 'md' as ExportFormat,
          path: savePath,
        });
        message.success('脱敏结果已导出为 Markdown 文件');
      }
    } catch (err) {
      message.error('导出失败: ' + err);
    } finally {
      setExportMdLoading(false);
    }
  };

  /// 统计清单中活跃项（未排除）数量
  const activeCount = scanItems.filter((i) => !i.excluded).length;
  const totalCount = scanItems.length;

  /// 清单表格列定义
  const scanColumns: ColumnsType<ScanItem> = [
    {
      title: '脱敏',
      dataIndex: 'excluded',
      key: 'excluded',
      width: 56,
      align: 'center' as const,
      render: (_: boolean, record: ScanItem) => (
        <Tooltip title={record.excluded ? '已排除，点击恢复' : '将被脱敏，点击排除'}>
          <Checkbox
            checked={!record.excluded}
            onChange={() => toggleExclude(record.id)}
          />
        </Tooltip>
      ),
    },
    {
      title: (
        <span className="flex items-center gap-1">
          原文
          <Switch
            size="small"
            checked={showOriginalInList}
            onChange={setShowOriginalInList}
            checkedChildren={<EyeOutlined />}
            unCheckedChildren={<EyeInvisibleOutlined />}
          />
        </span>
      ),
      dataIndex: 'originalValue',
      key: 'originalValue',
      render: (val: string, record: ScanItem) => (
        <Text
          className={`text-xs ${record.excluded ? 'line-through text-gray-400' : 'text-red-600'}`}
        >
          {showOriginalInList ? val : '******'}
        </Text>
      ),
    },
    {
      title: '识别依据',
      dataIndex: 'entityName',
      key: 'entityName',
      width: 120,
      render: (val: string, record: ScanItem) => (
        <Tag color={record.excluded ? 'default' : 'geekblue'}>{val}</Tag>
      ),
    },
    {
      title: (
        <Tooltip title="该词在原文中出现的次数，即本次将被替换的次数">
          <span style={{ cursor: 'help' }}>次数</span>
        </Tooltip>
      ),
      dataIndex: 'count',
      key: 'count',
      width: 64,
      align: 'center' as const,
      render: (val: number, record: ScanItem) => (
        <Tag color={record.excluded ? 'default' : (val > 1 ? 'orange' : 'default')} style={{ fontSize: 11 }}>
          {val}
        </Tag>
      ),
    },
    {
      title: '策略',
      dataIndex: 'strategy',
      key: 'strategy',
      width: 100,
      render: (val: 'random_replace' | 'empty', record: ScanItem) => (
        <Select
          size="small"
          value={val}
          style={{ width: 96 }}
          disabled={record.excluded}
          onChange={(newVal: 'random_replace' | 'empty') => {
            setScanItems(
              scanItems.map((item) =>
                item.id === record.id ? { ...item, strategy: newVal } : item
              )
            );
          }}
          options={[
            { value: 'random_replace', label: '替换' },
            { value: 'empty', label: '置空' },
          ]}
        />
      ),
    },
    {
      title: '',
      key: 'action',
      width: 48,
      render: (_: unknown, record: ScanItem) => (
        <Tooltip title="从清单移除">
          <Button
            type="text"
            size="small"
            danger
            onClick={() => removeItem(record.id)}
          >
            ✕
          </Button>
        </Tooltip>
      ),
    },
  ];

  /// 清单表格列定义
  const getRecommendedFormat = (): ExportFormat => {
    if (!loadedFile) return 'docx';
    const typeMap: Record<string, ExportFormat> = {
      docx: 'docx',
      doc: 'docx',
      xlsx: 'xlsx',
      xls: 'xlsx',
      pdf: 'pdf',
      txt: 'txt',
      log: 'txt',
    };
    return typeMap[loadedFile.fileType] || 'docx';
  };

  const getSuggestedFilename = (): string => {
    if (!loadedFile) return `还原结果_${new Date().toISOString().slice(0, 10)}`;
    return loadedFile.filename.replace(/\.[^.]+$/, '') + '_还原';
  };

  return (
    <Layout style={{ height: '100%', background: '#fff' }}>
      {/* 主操作区域 */}
      <Content style={{ padding: '24px', overflowY: 'auto' }}>
        <Title level={3} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <ThunderboltOutlined style={{ color: 'var(--primary-color)', fontSize: 22 }} />
          文案脱敏
        </Title>

        {/* 风险提示 */}
        <Alert
          type="warning"
          showIcon
          className="mb-4"
          message="识别准确率提示"
          description={
            <span>
              本地版受设备性能与资源限制，内置模型识别能力有限，<strong>可能存在漏识别情况</strong>，请在执行脱敏前仔细审查清单。
              如发现遗漏项，可在识别结果清单中手动新增，也可前往左侧
              <strong>「敏感实体策略」</strong>将常用词条加入自定义分组，方便后续自动识别。
            </span>
          }
        />

        {/* 输入区域 */}
        <Card
          title="输入原始内容"
          className="mb-4"
        >
          <div className="mb-3">
            <FileUpload onFileLoad={handleFileLoad} />
          </div>
          <TextArea
            value={inputContent}
            onChange={(e) => setInputContent(e.target.value)}
            placeholder="在此粘贴需要脱敏的文本内容，或通过上方上传文件自动填充..."
            rows={7}
            className="font-mono text-sm"
          />
          <div className="mt-4 text-center">
            <Button
              type="primary"
              icon={<ScanOutlined />}
              onClick={handleScan}
              loading={scanLoading}
              size="large"
              disabled={!inputContent.trim()}
            >
              识别敏感词
            </Button>
            {nerStatus && (
              <div className="mt-2">
                {nerStatus.ready ? (
                  <Text type="success" className="text-xs">
                    ✓ NER 模型已就绪，支持自动识别姓名/公司/地址
                  </Text>
                ) : nerStatus.loading ? (
                  <Text type="secondary" className="text-xs">
                    NER 模型加载中，姓名/公司/地址识别稍后可用…
                  </Text>
                ) : nerStatus.error ? (
                  <Text type="danger" className="text-xs">
                    NER 模型加载失败（{nerStatus.error}），姓名/公司/地址可能无法自动识别
                  </Text>
                ) : null}
              </div>
            )}
          </div>
        </Card>

        {/* 阶段二：敏感词清单编辑区（扫描后显示） */}
        {(scanItems.length > 0 || scanLoading) && (
          <Card
            title={
              <Space size={8}>
                <SafetyOutlined style={{ color: 'var(--primary-color)' }} />
                <span>识别到的敏感词清单</span>
                <Badge
                  count={`${activeCount} / ${totalCount}`}
                  style={{ backgroundColor: activeCount > 0 ? 'var(--primary-color)' : '#d9d9d9' }}
                />
              </Space>
            }
            className="mb-4"
            extra={
              <Space>
                <Tooltip title="反转所有词条的勾选状态">
                  <Button
                    size="small"
                    onClick={invertSelection}
                  >
                    反选
                  </Button>
                </Tooltip>
                <Tooltip title="新增本次未识别但需要脱敏的词条">
                  <Button
                    icon={<PlusOutlined />}
                    size="small"
                    onClick={() => setAddModalOpen(true)}
                  >
                    新增词条
                  </Button>
                </Tooltip>
                <Button
                  type="primary"
                  icon={hasResult ? <ReloadOutlined /> : <ThunderboltOutlined />}
                  onClick={handleDesensitize}
                  loading={loading}
                  size="small"
                  disabled={activeCount === 0}
                >
                  {hasResult ? '重新脱敏' : '执行脱敏'}
                </Button>
              </Space>
            }
          >
            <div className="mb-2">
              <Text type="secondary" className="text-xs">
                取消勾选可排除该词条（保留原文不替换）；可修改替换策略或新增遗漏词条，再点击「执行脱敏」
              </Text>
            </div>
            <Table
              columns={scanColumns}
              dataSource={scanItems}
              rowKey="id"
              size="small"
              pagination={false}
              className="border rounded"
              rowClassName={(record) => record.excluded ? 'opacity-40' : ''}
            />
            {activeCount === 0 && totalCount > 0 && (
              <div className="mt-2 text-center">
                <Text type="warning" className="text-xs">所有词条已排除，执行脱敏将不做任何替换</Text>
              </div>
            )}
          </Card>
        )}

        {/* 阶段三：脱敏结果 */}
        {desensitizedContent && (
          <Card
            title="脱敏结果（Markdown 格式）"
            className="mb-4"
            extra={
              <Space>
                <Tooltip title="复制脱敏内容到剪贴板">
                  <Button
                    icon={<CopyOutlined />}
                    onClick={() => handleCopy(desensitizedContent)}
                    size="small"
                  >
                    复制
                  </Button>
                </Tooltip>
                <Tooltip title="将脱敏结果保存为 .md 文件">
                  <Button
                    icon={<FileMarkdownOutlined />}
                    onClick={handleExportMd}
                    loading={exportMdLoading}
                    size="small"
                  >
                    导出为 MD
                  </Button>
                </Tooltip>
              </Space>
            }
          >
            <TextArea
              value={desensitizedContent}
              readOnly
              rows={7}
              className="bg-gray-50 font-mono text-sm"
            />
            {currentSessionId && (
              <div className="mt-2">
                <Text type="secondary" className="text-xs">
                  会话ID: {currentSessionId}
                </Text>
              </div>
            )}
          </Card>
        )}

        <Divider />

        {/* 还原区域 */}
        <Card title="还原 AI 返回的内容" className="mb-4">
          <TextArea
            value={restoreContent}
            onChange={(e) => setRestoreContent(e.target.value)}
            placeholder="在此粘贴 AI 返回的包含占位符的内容（如 [姓名_1]、[邮箱_2] 等）..."
            rows={5}
            className="font-mono text-sm"
          />
          <div className="mt-4 text-center">
            <Button
              type="primary"
              icon={<RollbackOutlined />}
              onClick={handleRestore}
              loading={loading}
              disabled={!restoreContent.trim() || !currentSessionId}
            >
              还原
            </Button>
          </div>
        </Card>

        {/* 还原结果 */}
        {restoredContent && (
          <Card
            title="还原结果"
            extra={
              <Space>
                <Button
                  icon={<CopyOutlined />}
                  onClick={() => handleCopy(restoredContent)}
                  size="small"
                >
                  复制
                </Button>
                <ExportButton
                  content={restoredContent}
                  defaultFormat={getRecommendedFormat()}
                  suggestedFilename={getSuggestedFilename()}
                  label="导出"
                />
              </Space>
            }
          >
            <TextArea
              value={restoredContent}
              readOnly
              rows={7}
              className="bg-gray-50 font-mono text-sm"
            />
          </Card>
        )}
      </Content>

      {/* 右侧会话列表 */}
      <Sider
        width={260}
        theme="light"
        style={{
          borderLeft: '1px solid #f0f0f0',
          background: '#fff',
          height: '100%',
          overflowY: 'auto',
        }}
      >
        <SessionList onSelect={handleSessionSelect} onNewSession={handleNewSession} />
      </Sider>

      {/* 新增词条弹窗 */}
      <AddItemsModal
        open={addModalOpen}
        onAdd={handleAddItems}
        onClose={() => setAddModalOpen(false)}
      />
    </Layout>
  );
}
