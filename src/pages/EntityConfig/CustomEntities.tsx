import { useState } from 'react';
import { Table, Button, Switch, Tag, Popconfirm, Space, message, Empty } from 'antd';
import { PlusOutlined, EditOutlined, DeleteOutlined } from '@ant-design/icons';
import type { Entity } from '../../types/entity';
import { useEntityStore } from '../../stores/entityStore';
import type { CreateEntityDto, UpdateEntityDto } from '../../services/entityApi';
import EntityForm from './EntityForm';

interface CustomEntitiesProps {
  entities: Entity[];
}

export default function CustomEntities({ entities }: CustomEntitiesProps) {
  const { createEntity, updateEntity, deleteEntity, toggleEntity } = useEntityStore();
  const [formOpen, setFormOpen] = useState(false);
  const [editingEntity, setEditingEntity] = useState<Entity | null>(null);
  const [submitLoading, setSubmitLoading] = useState(false);

  const handleAdd = () => {
    setEditingEntity(null);
    setFormOpen(true);
  };

  const handleEdit = (record: Entity) => {
    setEditingEntity(record);
    setFormOpen(true);
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteEntity(id);
      message.success('实体已删除');
    } catch (err) {
      message.error(String(err));
    }
  };

  const handleToggle = async (id: string, enabled: boolean) => {
    try {
      await toggleEntity(id, enabled);
    } catch (err) {
      message.error(String(err));
    }
  };

  const handleSubmit = async (values: CreateEntityDto | UpdateEntityDto) => {
    setSubmitLoading(true);
    try {
      if (editingEntity) {
        await updateEntity(values as UpdateEntityDto);
        message.success('实体已更新');
      } else {
        await createEntity(values as CreateEntityDto);
        message.success('实体已创建');
      }
      setFormOpen(false);
    } catch (err) {
      message.error(String(err));
    } finally {
      setSubmitLoading(false);
    }
  };

  const columns = [
    {
      title: '实体名称',
      dataIndex: 'name',
      key: 'name',
    },
    {
      title: '匹配词',
      dataIndex: 'synonyms',
      key: 'synonyms',
      render: (synonyms: string[]) => {
        if (!synonyms || synonyms.length === 0) return <span className="text-gray-400">-</span>;
        return (
          <span className="text-xs text-gray-500">
            {synonyms.slice(0, 3).join('、')}
            {synonyms.length > 3 ? `...等${synonyms.length}项` : ''}
          </span>
        );
      },
    },
    {
      title: '替换策略',
      dataIndex: 'strategy',
      key: 'strategy',
      render: (strategy: string) =>
        strategy === 'random_replace' ? (
          <Tag color="blue">随机替换</Tag>
        ) : (
          <Tag color="orange">置空</Tag>
        ),
    },
    {
      title: '生效',
      dataIndex: 'enabled',
      key: 'enabled',
      render: (enabled: boolean, record: Entity) => (
        <Switch
          checked={enabled}
          size="small"
          onChange={(checked) => handleToggle(record.id, checked)}
        />
      ),
    },
    {
      title: '操作',
      key: 'action',
      render: (_: unknown, record: Entity) => (
        <Space size="small">
          <Button
            type="link"
            icon={<EditOutlined />}
            size="small"
            onClick={() => handleEdit(record)}
          >
            编辑
          </Button>
          <Popconfirm
            title="确定删除该实体吗？"
            description="删除后不可恢复，相关脱敏记录不受影响。"
            onConfirm={() => handleDelete(record.id)}
            okText="确定删除"
            cancelText="取消"
            okButtonProps={{ danger: true }}
          >
            <Button
              type="link"
              icon={<DeleteOutlined />}
              size="small"
              danger
            >
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <>
      <div className="mb-3">
        <Button
          type="primary"
          icon={<PlusOutlined />}
          onClick={handleAdd}
        >
          新增实体
        </Button>
      </div>

      {entities.length === 0 ? (
        <Empty
          description={
            <span className="text-gray-400">
              暂无自定义实体，点击「新增实体」添加业务专属敏感词
            </span>
          }
          className="py-8"
        />
      ) : (
        <Table
          dataSource={entities}
          columns={columns}
          rowKey="id"
          pagination={false}
          size="middle"
        />
      )}

      <EntityForm
        open={formOpen}
        entity={editingEntity}
        onSubmit={handleSubmit}
        onCancel={() => setFormOpen(false)}
        confirmLoading={submitLoading}
      />
    </>
  );
}
