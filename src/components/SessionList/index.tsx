import { useEffect } from 'react';
import { List, Button, Tag, Typography, Spin, Modal, Tooltip } from 'antd';
import { DeleteOutlined, HistoryOutlined, ClearOutlined, PlusOutlined } from '@ant-design/icons';
import { useSessionStore } from '../../stores/sessionStore';
import type { SessionListItem } from '../../types/session';

const { Text } = Typography;

interface SessionListProps {
  onSelect?: (sessionId: string) => void;
  onNewSession?: () => void;
}

export default function SessionList({ onSelect, onNewSession }: SessionListProps) {
  const {
    sessions,
    sessionsLoading,
    currentSessionId,
    fetchSessions,
    loadSession,
    deleteSession,
    clearAllSessions,
  } = useSessionStore();

  useEffect(() => {
    fetchSessions();
  }, []);

  /// 点击会话项加载历史数据
  const handleSelect = async (session: SessionListItem) => {
    if (session.id === currentSessionId) return;
    await loadSession(session.id);
    onSelect?.(session.id);
  };

  /// 删除单个会话
  const handleDelete = (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    Modal.confirm({
      title: '删除会话',
      content: '确定要删除此会话吗？删除后无法恢复。',
      okText: '删除',
      okType: 'danger',
      cancelText: '取消',
      onOk: () => deleteSession(id),
    });
  };

  /// 清空所有会话
  const handleClearAll = () => {
    if (sessions.length === 0) return;
    Modal.confirm({
      title: '清空所有会话',
      content: `确定要清空全部 ${sessions.length} 条会话记录吗？此操作不可撤销。`,
      okText: '清空',
      okType: 'danger',
      cancelText: '取消',
      onOk: () => clearAllSessions(),
    });
  };

  return (
    <div className="session-list">
      {/* 标题栏 */}
      <div className="session-list-header">
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <HistoryOutlined style={{ color: 'var(--text-secondary)' }} />
          <Text strong style={{ fontSize: 13 }}>会话记录</Text>
          {sessions.length > 0 && (
            <Tag style={{ marginLeft: 2, fontSize: 11 }}>{sessions.length}</Tag>
          )}
        </div>
        <div style={{ display: 'flex', gap: 4 }}>
          <Tooltip title="新增会话">
            <Button
              size="small"
              icon={<PlusOutlined />}
              onClick={onNewSession}
              type="text"
            />
          </Tooltip>
          <Tooltip title="清空所有会话">
            <Button
              size="small"
              danger
              icon={<ClearOutlined />}
              onClick={handleClearAll}
              disabled={sessions.length === 0}
              type="text"
            />
          </Tooltip>
        </div>
      </div>

      {/* 列表区域 */}
      <div style={{ flex: 1, overflowY: 'auto' }}>
        <Spin spinning={sessionsLoading}>
          {sessions.length === 0 && !sessionsLoading ? (
            <div className="empty-state">
              <HistoryOutlined className="empty-state-icon" />
              <span className="empty-state-text">暂无会话记录</span>
              <Text type="secondary" style={{ fontSize: 12, textAlign: 'center' }}>
                执行脱敏操作后，会话将自动保存在这里
              </Text>
            </div>
          ) : (
            <List
              dataSource={sessions}
              renderItem={(session) => (
                <List.Item
                  className={`session-item ${session.id === currentSessionId ? 'active' : ''}`}
                  style={{ display: 'block', padding: 0 }}
                  onClick={() => handleSelect(session)}
                >
                  <div style={{ padding: '10px 16px', position: 'relative' }}>
                    <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 4 }}>
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <div className="session-item-name">{session.name}</div>
                        {session.preview && (
                          <div className="session-item-meta" style={{ marginTop: 2, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                            {session.preview}
                          </div>
                        )}
                        <div style={{ marginTop: 4 }}>
                          <Tag color="blue" style={{ fontSize: 11 }}>
                            {session.mappingCount} 个脱敏项
                          </Tag>
                        </div>
                      </div>
                      <Tooltip title="删除此会话">
                        <Button
                          size="small"
                          type="text"
                          danger
                          icon={<DeleteOutlined />}
                          onClick={(e) => handleDelete(e, session.id)}
                          style={{ flexShrink: 0, opacity: 0.6 }}
                        />
                      </Tooltip>
                    </div>
                  </div>
                </List.Item>
              )}
            />
          )}
        </Spin>
      </div>
    </div>
  );
}
