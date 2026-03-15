import { create } from 'zustand';
import type { Entity } from '../types/entity';
import { entityApi } from '../services/entityApi';
import type { CreateEntityDto, UpdateEntityDto } from '../services/entityApi';

interface EntityState {
  entities: Entity[];
  loading: boolean;
  error: string | null;
  fetchEntities: () => Promise<void>;
  toggleEntity: (id: string, enabled: boolean) => Promise<void>;
  updateEntityStrategy: (id: string, strategy: string) => Promise<void>;
  createEntity: (dto: CreateEntityDto) => Promise<Entity>;
  updateEntity: (dto: UpdateEntityDto) => Promise<Entity>;
  deleteEntity: (id: string) => Promise<void>;
}

export const useEntityStore = create<EntityState>((set, get) => ({
  entities: [],
  loading: false,
  error: null,

  fetchEntities: async () => {
    set({ loading: true, error: null });
    try {
      const entities = await entityApi.getAll();
      set({ entities, loading: false });
    } catch (err) {
      set({ error: String(err), loading: false });
    }
  },

  toggleEntity: async (id: string, enabled: boolean) => {
    try {
      await entityApi.toggle(id, enabled);
      const { entities } = get();
      set({
        entities: entities.map(e =>
          e.id === id ? { ...e, enabled } : e
        )
      });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  updateEntityStrategy: async (id: string, strategy: string) => {
    try {
      await entityApi.updateStrategy(id, strategy);
      const { entities } = get();
      set({
        entities: entities.map(e =>
          e.id === id ? { ...e, strategy: strategy as Entity['strategy'] } : e
        )
      });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  createEntity: async (dto: CreateEntityDto) => {
    try {
      const newEntity = await entityApi.create(dto);
      const { entities } = get();
      set({ entities: [...entities, newEntity] });
      return newEntity;
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  updateEntity: async (dto: UpdateEntityDto) => {
    try {
      const updated = await entityApi.update(dto);
      const { entities } = get();
      set({
        entities: entities.map(e => e.id === dto.id ? { ...e, ...updated } : e)
      });
      return updated;
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  deleteEntity: async (id: string) => {
    try {
      await entityApi.delete(id);
      const { entities } = get();
      set({ entities: entities.filter(e => e.id !== id) });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },
}));
