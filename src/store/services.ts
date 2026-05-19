import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { SERVICES, type ServiceDefinition } from "@/services/serviceRegistry";

interface ServicesState {
  /** Currently visible service id, or null */
  activeId: string | null;
  /** Whether the service picker flyout is open */
  flyoutOpen: boolean;
  /** True while open_service command is in-flight */
  isLoading: boolean;

  openService: (service: ServiceDefinition) => Promise<void>;
  closeService: () => Promise<void>;
  toggleFlyout: () => void;
  closeFlyout: () => void;
}

export const useServicesStore = create<ServicesState>((set, get) => ({
  activeId: null,
  flyoutOpen: false,
  isLoading: false,

  toggleFlyout: () => {
    const { flyoutOpen, activeId } = get();
    const opening = !flyoutOpen;
    set({ flyoutOpen: opening });
    if (activeId) {
      invoke(opening ? "hide_service_view" : "show_service_view").catch((e) =>
        console.error("flyout toggle error:", e),
      );
    }
  },

  closeFlyout: () => {
    const { activeId } = get();
    set({ flyoutOpen: false });
    if (activeId) {
      invoke("show_service_view").catch((e) =>
        console.error("show_service_view error:", e),
      );
    }
  },

  openService: async (service) => {
    // Optimistic update: title and loading bar appear immediately.
    set({ activeId: service.id, flyoutOpen: false, isLoading: true });
    try {
      await invoke("open_service", { serviceId: service.id, url: service.url });
      console.log("[ingwe] open_service succeeded:", service.id);
    } catch (e) {
      console.error("[ingwe] open_service failed:", e);
    } finally {
      set({ isLoading: false });
    }
  },

  closeService: async () => {
    await invoke("close_service");
    set({ activeId: null });
  },
}));

export { SERVICES };
