import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { TitleBar } from "@/components/TitleBar";
import { Sidebar } from "@/components/Sidebar";
import { WebviewMount } from "@/components/WebviewMount";
import { ServiceWizard } from "@/components/ServiceWizard";
import { useServicesStore } from "@/store/services";

function App() {
  const initFromStore = useServicesStore((s) => s.initFromStore);
  const setFullscreen = useServicesStore((s) => s.setFullscreen);
  const wizardOpen = useServicesStore((s) => s.wizardOpen);

  useEffect(() => {
    initFromStore();

    let unlisten: (() => void) | undefined;
    listen<boolean>("fullscreen-changed", (e) => {
      setFullscreen(e.payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  return (
    <div className="flex flex-col h-screen bg-bg-base text-text-primary overflow-hidden">
      <TitleBar />
      <div className="relative flex-1 overflow-hidden">
        <WebviewMount />
        <Sidebar />
      </div>
      {wizardOpen && <ServiceWizard />}
    </div>
  );
}

export default App;
