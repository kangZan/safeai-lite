import { useState } from 'react';
import { Table, Switch, Collapse, Tag, Typography, Empty } from 'antd';
import { EyeOutlined, EyeInvisibleOutlined } from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import type { MappingItem } from '../../types/session';

const { Text } = Typography;

interface MappingTableProps {
  mappings: MappingItem[];
}

export default function MappingTable({ mappings }: MappingTableProps) {
  const [showOriginal, setShowOriginal] = useState(false);

  if (mappings.length === 0) {
    return null;
  }

  const columns: ColumnsType<MappingItem> = [
    {
      title: '占位符',
      dataIndex: 'placeholder',
      key: 'placeholder',
      width: 140,
      render: (value: string) => (
        <Text code className="text-xs">{value}</Text>
      ),
    },
    {
      title: (
        <span className="flex items-center gap-1">
          原始值
          <Switch
            size="small"
            checked={showOriginal}
            onChange={setShowOriginal}
            checkedChildren={<EyeOutlined />}
            unCheckedChildren={<EyeInvisibleOutlined />}
          />
        </span>
      ),
      dataIndex: 'originalValue',
      key: 'originalValue',
      render: (value: string) =>
        showOriginal ? (
          <Text className="text-xs text-red-600">{value}</Text>
        ) : (
          <Text className="text-xs text-gray-400">******</Text>
        ),
    },
    {
      title: '实体类型',
      dataIndex: 'entityName',
      key: 'entityName',
      width: 120,
      render: (value: string) => (
        <Tag color="geekblue" className="text-xs">{value}</Tag>
      ),
    },
  ];

  const header = (
    <span className="flex items-center gap-2">
      <span>脱敏映射表</span>
      <Tag color="blue">{mappings.length} 项</Tag>
      {!showOriginal && (
        <Text type="secondary" className="text-xs font-normal">
          （原始值已隐藏，点击表头开关可显示）
        </Text>
      )}
    </span>
  );

  return (
    <Collapse
      size="small"
      className="mt-3"
      items={[
        {
          key: 'mapping',
          label: header,
          children:
            mappings.length === 0 ? (
              <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无映射数据" />
            ) : (
              <Table
                columns={columns}
                dataSource={mappings}
                rowKey="id"
                size="small"
                pagination={false}
                className="mapping-table"
              />
            ),
        },
      ]}
    />
  );
}
