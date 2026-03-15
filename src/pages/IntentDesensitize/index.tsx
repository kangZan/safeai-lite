import { LockOutlined } from '@ant-design/icons';
import { Card, Tag, Typography } from 'antd';

const { Title, Paragraph } = Typography;

export default function IntentDesensitize() {
  return (
    <div className="p-6 flex items-center justify-center min-h-[60vh]">
      <Card className="text-center max-w-md">
        <LockOutlined style={{ fontSize: 64, color: '#d9d9d9' }} />
        <Title level={4} className="mt-4">意图脱敏</Title>
        <Paragraph className="text-gray-500">
          智能识别文本意图，进行语义级别的脱敏处理
        </Paragraph>
        <Tag color="blue">基础版暂不支持，敬请期待</Tag>
      </Card>
    </div>
  );
}
