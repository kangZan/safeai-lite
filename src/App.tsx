import { Suspense, lazy } from 'react';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import { Spin } from 'antd';
import { SafetyOutlined } from '@ant-design/icons';
import Sidebar from './components/Sidebar';

// 路由级懒加载（代码分割）
const EntityConfig = lazy(() => import('./pages/EntityConfig'));
const Desensitize = lazy(() => import('./pages/Desensitize'));
const BatchDesensitize = lazy(() => import('./pages/BatchDesensitize'));
const IntentDesensitize = lazy(() => import('./pages/IntentDesensitize'));
const ProxySettings = lazy(() => import('./pages/ProxySettings'));

const PageLoader = () => (
  <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100%', padding: '48px' }}>
    <Spin size="large" />
  </div>
);

function App() {
  return (
    <Router>
      <div className="app-layout">
        {/* 左侧导航栏 */}
        <aside className="app-sider" style={{ width: 200 }}>
          <div className="app-logo">
            <div className="app-logo-icon">
              <SafetyOutlined style={{ fontSize: 14, color: '#fff' }} />
            </div>
            <span className="app-logo-text">SafeAI-Lite</span>
          </div>
          <Sidebar />
        </aside>

        {/* 右侧内容区 */}
        <main className="app-content">
          <div className="app-main">
            <Suspense fallback={<PageLoader />}>
              <Routes>
                <Route path="/" element={<Navigate to="/desensitize" replace />} />
                <Route path="/desensitize" element={<Desensitize />} />
                <Route path="/batch" element={<BatchDesensitize />} />
                <Route path="/entity-config" element={<EntityConfig />} />
                <Route path="/intent-desensitize" element={<IntentDesensitize />} />
                <Route path="/proxy-settings" element={<ProxySettings />} />
              </Routes>
            </Suspense>
          </div>
        </main>
      </div>
    </Router>
  );
}

export default App;
