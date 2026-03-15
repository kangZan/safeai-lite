import { useLocation, useNavigate } from 'react-router-dom';
import { Menu, Tooltip } from 'antd';
import {
  ThunderboltOutlined,
  SafetyOutlined,
  LockOutlined,
  SettingOutlined,
} from '@ant-design/icons';

const COMING_SOON = '基础版暂不支持，敬请期待';

const items = [
  {
    key: '/desensitize',
    icon: <ThunderboltOutlined />,
    label: '文案脱敏',
    disabled: false,
  },
  {
    key: '/entity-config',
    icon: <SafetyOutlined />,
    label: '敏感实体策略',
    disabled: false,
  },
  {
    key: '/intent-desensitize',
    icon: <LockOutlined />,
    label: (
      <Tooltip title={COMING_SOON} placement="right">
        <span style={{ opacity: 0.5 }}>意图脱敏</span>
      </Tooltip>
    ),
    disabled: true,
  },
  {
    key: '/proxy-settings',
    icon: <SettingOutlined />,
    label: (
      <Tooltip title={COMING_SOON} placement="right">
        <span style={{ opacity: 0.5 }}>代理设置</span>
      </Tooltip>
    ),
    disabled: true,
  },
];

export default function Sidebar() {
  const location = useLocation();
  const navigate = useNavigate();

  return (
    <Menu
      mode="inline"
      theme="dark"
      selectedKeys={[location.pathname]}
      items={items}
      onClick={({ key }) => {
        const item = items.find((i) => i.key === key);
        if (!item?.disabled) {
          navigate(key);
        }
      }}
      className="app-menu"
      style={{ borderRight: 0, background: 'transparent' }}
    />
  );
}
