import { SettingOutlined } from '@ant-design/icons';
import { Card, Tag, Typography } from 'antd';

const { Title, Paragraph } = Typography;

export default function ProxySettings() {
  return (
    <div className="p-6 flex items-center justify-center min-h-[60vh]">
      <Card className="text-center max-w-md">
        <SettingOutlined style={{ fontSize: 64, color: '#d9d9d9' }} />
        <Title level={4} className="mt-4">代理设置</Title>
        <Paragraph className="text-gray-500">
          配置AI服务代理，支持更多AI平台接入
        </Paragraph>
        <Tag color="blue">基础版暂不支持，敬请期待</Tag>
      </Card>
    </div>
  );
}
