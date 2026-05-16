import { TitleBar } from "@/components/TitleBar";
import { Sidebar } from "@/components/Sidebar";
import { WebviewMount } from "@/components/WebviewMount";

function App() {
  return (
    <div className="flex flex-col h-screen bg-bg-base text-text-primary overflow-hidden">
      <TitleBar />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar />
        <WebviewMount />
      </div>
    </div>
  );
}

export default App;

