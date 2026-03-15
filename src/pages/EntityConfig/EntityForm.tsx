import { useEffect } from 'react';
import { Modal, Form, Input, Select, Switch, Tooltip } from 'antd';
import { QuestionCircleOutlined } from '@ant-design/icons';
import type { Entity } from '../../types/entity';
import type { CreateEntityDto, UpdateEntityDto } from '../../services/entityApi';

const { TextArea } = Input;

interface EntityFormProps {
  open: boolean;
  entity?: Entity | null;
  initialName?: string;  // 新增时预填充实体名称
  onSubmit: (values: CreateEntityDto | UpdateEntityDto) => Promise<void>;
  onCancel: () => void;
  confirmLoading?: boolean;
}

const strategyOptions = [
  { value: 'random_replace', label: '随机数据替换（可还原）' },
  { value: 'empty', label: '置空（不可还原）' },
];

// 前端正则校验
function validateRegex(_: unknown, value: string) {
  if (!value || value.trim() === '') return Promise.resolve();
  try {
    new RegExp(value);
    return Promise.resolve();
  } catch {
    return Promise.reject(new Error('正则表达式格式无效'));
  }
}

export default function EntityForm({
  open,
  entity,
  initialName,
  onSubmit,
  onCancel,
  confirmLoading,
}: EntityFormProps) {
  const [form] = Form.useForm();
  const isEdit = !!entity;

  useEffect(() => {
    if (open) {
      if (entity) {
        form.setFieldsValue({
          name: entity.name,
          synonyms: entity.synonyms.join('\n'),
          regex_pattern: entity.regexPattern ?? '',
          strategy: entity.strategy,
          enabled: entity.enabled,
        });
      } else {
        form.resetFields();
        form.setFieldsValue({
          strategy: 'random_replace',
          enabled: true,
          ...(initialName ? { name: initialName } : {}),
        });
      }
    }
  }, [open, entity, initialName, form]);

  const handleFinish = async (values: {
    name: string;
    synonyms?: string;
    regex_pattern?: string;
    strategy: 'random_replace' | 'empty';
    enabled: boolean;
  }) => {
    // 解析匹配词：按换行分割，过滤空行
    const synonyms = (values.synonyms ?? '')
      .split('\n')
      .map((s: string) => s.trim())
      .filter((s: string) => s.length > 0);

    const regexPattern = values.regex_pattern?.trim() || undefined;

    if (isEdit && entity) {
      await onSubmit({
        id: entity.id,
        name: values.name.trim(),
        synonyms,
        regexPattern,
        strategy: values.strategy,
        enabled: values.enabled,
      } as UpdateEntityDto);
    } else {
      await onSubmit({
        name: values.name.trim(),
        synonyms,
        regexPattern,
        strategy: values.strategy,
        enabled: values.enabled,
      } as CreateEntityDto);
    }
  };

  return (
    <Modal
      title={isEdit ? '编辑自定义实体' : '新增自定义实体'}
      open={open}
      onOk={() => form.submit()}
      onCancel={onCancel}
      confirmLoading={confirmLoading}
      okText={isEdit ? '保存' : '创建'}
      cancelText="取消"
      destroyOnClose
      width={520}
    >
      <Form
        form={form}
        layout="vertical"
        onFinish={handleFinish}
      >
        <Form.Item
          name="name"
          label="实体名称"
          rules={[
            { required: true, message: '请输入实体名称' },
            { max: 20, message: '不能超过20个字符' },
          ]}
        >
          <Input placeholder="如：项目代号" maxLength={20} showCount />
        </Form.Item>

        <Form.Item
          name="synonyms"
          label={
            <span>
              匹配词列表&nbsp;
              <Tooltip title="每行输入一个需要识别的词语或词组，例如真实人名、项目名、公司名等，工具将逐一匹配并脱敏">
                <QuestionCircleOutlined className="text-gray-400" />
              </Tooltip>
            </span>
          }
          extra="每行一个词语，如无正则时需填写至少一项，例如人名可逐行填写各个姓名"
        >
          <TextArea
            rows={4}
            placeholder={'张三\n李四\n王五'}
          />
        </Form.Item>

        <Form.Item
          name="regex_pattern"
          label={
            <span>
              正则表达式（可选）&nbsp;
              <Tooltip title="用于精确匹配复杂模式，例如 PROJ-\d{4}-\d{3}。不熟悉正则可留空，仅用匹配词列表识别。">
                <QuestionCircleOutlined className="text-gray-400" />
              </Tooltip>
            </span>
          }
          rules={[{ validator: validateRegex }]}
        >
          <Input placeholder={String.raw`如：PROJ-\d{4}-\d{3}`} />
        </Form.Item>

        <Form.Item
          name="strategy"
          label="替换策略"
          rules={[{ required: true, message: '请选择替换策略' }]}
        >
          <Select options={strategyOptions} />
        </Form.Item>

        <Form.Item name="enabled" label="立即生效" valuePropName="checked">
          <Switch />
        </Form.Item>
      </Form>
    </Modal>
  );
}
