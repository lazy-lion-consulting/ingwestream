import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { SERVICES, type ServiceDefinition } from "@/services/serviceRegistry";

interface ServicesState {
  /** Services that have been opened (webview created in Rust) */
  loaded: Set<string>;
  /** Currently visible service id, or null */
  activeId: string | null;

  openService: (service: ServiceDefinition) => Promise<void>;
  switchService: (id: string) => Promise<void>;
  closeService: (id: string) => Promise<void>;
}

export const useServicesStore = create<ServicesState>((set, get) => ({
  loaded: new Set(),
  activeId: null,

  openService: async (service) => {
    const { loaded, switchService } = get();

    if (!loaded.has(service.id)) {
      await invoke("open_service", { serviceId: service.id, url: service.url });
      set((s) => ({ loaded: new Set(s.loaded).add(service.id) }));
    }

    await switchService(service.id);
  },

  switchService: async (id) => {
    const { activeId } = get();
    if (activeId === id) return;

    await invoke("switch_service", { serviceId: id });
    set({ activeId: id });
  },

  closeService: async (id) => {
    await invoke("close_service", { serviceId: id });
    set((s) => {
      const next = new Set(s.loaded);
      next.delete(id);
      return {
        loaded: next,
        activeId: s.activeId === id ? null : s.activeId,
      };
    });
  },
}));

export { SERVICES };
