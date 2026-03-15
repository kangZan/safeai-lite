import { useEffect } from 'react';
import { Card, Table, Switch, Tag, Select, Tooltip, Typography, Spin, message, Badge } from 'antd';
import { InfoCircleOutlined, SafetyOutlined, AppstoreAddOutlined } from '@ant-design/icons';
import { useEntityStore } from '../../stores/entityStore';
import CustomEntities from './CustomEntities';
import type { Entity } from '../../types/entity';

const { Text } = Typography;

// 策略选项：只有基础版支持的两种可选，其他置灶
const STRATEGY_OPTIONS = [
  { value: 'random_replace', label: '随机数据替换' },
  { value: 'empty', label: '置空' },
  {
    value: 'proportional',
    label: (
      <Tooltip title="基础版暂不支持，敬请期待">
        <span style={{ color: '#ccc', cursor: 'not-allowed' }}>数字比例化（需要模型支持）</span>
      </Tooltip>
    ),
    disabled: true,
  },
  {
    value: 'fuzzy',
    label: (
      <Tooltip title="基础版暂不支持，敬请期待">
        <span style={{ color: '#ccc', cursor: 'not-allowed' }}>数字模糊化（需要模型支持）</span>
      </Tooltip>
    ),
    disabled: true,
  },
];

const STRATEGY_LABEL_MAP: Record<string, { text: string; color: string }> = {
  random_replace: { text: '随机替换', color: 'blue' },
  empty: { text: '置空', color: 'orange' },
};

export default function EntityConfig() {
  const { entities, loading, fetchEntities, toggleEntity, updateEntityStrategy } = useEntityStore();

  useEffect(() => {
    fetchEntities();
  }, [fetchEntities]);

  const handleStrategyChange = async (id: string, strategy: string) => {
    try {
      await updateEntityStrategy(id, strategy);
      message.success('策略已更新');
    } catch (err) {
      message.error(String(err));
    }
  };

  const builtinColumns = [
    {
      title: '实体名称',
      dataIndex: 'name',
      key: 'name',
      width: '30%',
      render: (name: string) => (
        <span style={{ fontWeight: 500, color: 'var(--text-primary)' }}>{name}</span>
      ),
    },
    {
      title: '替换策略',
      dataIndex: 'strategy',
      key: 'strategy',
      width: '40%',
      render: (strategy: string, record: Entity) => (
        <Select
          value={strategy}
          options={STRATEGY_OPTIONS}
          style={{ width: 220 }}
          size="small"
          onChange={(val) => handleStrategyChange(record.id, val)}
        />
      ),
    },
    {
      title: '当前策略',
      dataIndex: 'strategy',
      key: 'strategyTag',
      width: '15%',
      render: (strategy: string) => {
        const info = STRATEGY_LABEL_MAP[strategy] ?? { text: strategy, color: 'default' };
        return <Tag color={info.color}>{info.text}</Tag>;
      },
    },
    {
      title: '生效',
      dataIndex: 'enabled',
      key: 'enabled',
      width: '15%',
      render: (enabled: boolean, record: Entity) => {
        const isNameEntity = record.name === '姓名/用户名';
        if (isNameEntity) {
          return (
            <Tooltip title="内置 NER 模型对法律/合同文体的人名识别能力有限，暂不支持自动识别。如需使用，请在「敏感实体策略」中手动添加同义词后开启。">
              <Switch checked={false} size="small" disabled />
            </Tooltip>
          );
        }
        return (
          <Tooltip title={enabled ? '点击关闭该实体的脱敏' : '点击开启该实体的脱敏'}>
            <Switch
              checked={enabled}
              size="small"
              onChange={(checked) => toggleEntity(record.id, checked)}
            />
          </Tooltip>
        );
      },
    },
  ];

  const builtinEntities = entities.filter(e => e.entityType === 'builtin');
  const customEntities = entities.filter(e => e.entityType === 'custom');
  const enabledCount = entities.filter(e => e.enabled).length;

  return (
    <div className="page-container" style={{ minHeight: '100vh', background: 'var(--bg-base)' }}>
      {/* 页面头部 */}
      <div className="page-header">
        <h2 className="page-title">
          <SafetyOutlined style={{ color: 'var(--primary-color)', fontSize: 22 }} />
          敏感实体策略配置
        </h2>
        <p className="page-subtitle">
          配置内置与自定义的敏感实体，控制脱敏识别范围和替换策略&nbsp;·;&nbsp;当前共 <Text strong>{enabledCount}</Text> 个实体已生效
        </p>
      </div>

      {/* 内置敏感实体 */}
      <Card
        className="app-card mb-4"
        title={
          <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <SafetyOutlined style={{ color: 'var(--primary-color)' }} />
            内置敏感实体
            <Badge count={builtinEntities.length} color="blue" style={{ fontSize: 11 }} />
          </span>
        }
        style={{ marginBottom: 16 }}
      >
        <Spin spinning={loading}>
          <Table
            dataSource={builtinEntities}
            columns={builtinColumns}
            rowKey="id"
            pagination={false}
            size="middle"
            className="app-table"
            rowClassName="fade-in"
          />
        </Spin>

        {/* 注意事项 */}
        <div className="info-banner" style={{ marginTop: 12 }}>
          <InfoCircleOutlined style={{ marginTop: 1, flexShrink: 0 }} />
          <span>
            置灶策略（数字比例化、模糊化等）需要人工智能模型支持，基础版暂不开放。您可选择《随机数据替换》或《置空》作为脱敏策略。
            <Tag color="default" style={{ marginLeft: 8, fontSize: 11 }}>基础版暂不支持</Tag>
          </span>
        </div>
      </Card>

      {/* 自定义敏感实体 */}
      <Card
        className="app-card"
        title={
          <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <AppstoreAddOutlined style={{ color: '#52c41a' }} />
            自定义敏感实体
            {customEntities.length > 0 && (
              <Badge count={customEntities.length} color="green" style={{ fontSize: 11 }} />
            )}
          </span>
        }
      >
        <Spin spinning={loading}>
          <CustomEntities entities={customEntities} />
        </Spin>
      </Card>
    </div>
  );
}
