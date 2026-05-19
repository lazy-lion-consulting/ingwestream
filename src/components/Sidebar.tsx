import {
  Disc,
  Headphones,
  Music,
  Play,
  Radio,
  Tv2,
  Waves,
  type LucideProps,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { useServicesStore, SERVICES } from "@/store/services";
import type { ServiceDefinition } from "@/services/serviceRegistry";

// Map icon string → component so Sidebar stays tree-shakeable
const ICON_MAP: Record<string, React.FC<LucideProps>> = {
  Disc,
  Headphones,
  Music,
  Play,
  Radio,
  Tv2,
  Waves,
};

function ServiceIcon({ name, ...props }: { name: string } & LucideProps) {
  const Icon = ICON_MAP[name] ?? Music;
  return <Icon {...props} />;
}

function ServiceItem({
  service,
  isActive,
  isLoaded,
}: {
  service: ServiceDefinition;
  isActive: boolean;
  isLoaded: boolean;
}) {
  const openService = useServicesStore((s) => s.openService);

  return (
    <button
      onClick={() => openService(service)}
      title={service.label}
      className={cn(
        "w-full flex items-center gap-2.5 px-3 py-2 rounded-md text-sm transition-colors duration-150",
        "text-text-secondary hover:text-text-primary hover:bg-bg-elevated",
        isActive && "bg-bg-overlay text-text-primary",
      )}
    >
      <ServiceIcon
        name={service.icon}
        className={cn("size-4 shrink-0", isActive ? "text-accent" : "")}
      />
      <span className="truncate">{service.label}</span>
      {isLoaded && !isActive && (
        <span className="ml-auto size-1.5 rounded-full bg-accent-dim shrink-0" />
      )}
    </button>
  );
}

export function Sidebar() {
  const flyoutOpen = useServicesStore((s) => s.flyoutOpen);
  const closeFlyout = useServicesStore((s) => s.closeFlyout);
  const activeId = useServicesStore((s) => s.activeId);
  const loaded = useServicesStore((s) => s.loaded);

  return (
    <>
      {/* Backdrop */}
      <div
        className={cn(
          "absolute inset-0 z-20 bg-black/50 transition-opacity duration-200",
          flyoutOpen ? "opacity-100 pointer-events-auto" : "opacity-0 pointer-events-none",
        )}
        onClick={closeFlyout}
      />

      {/* Flyout panel */}
      <aside
        className={cn(
          "absolute left-0 top-0 bottom-0 w-52 flex flex-col",
          "bg-bg-surface border-r border-border-base z-30",
          "transition-transform duration-200 ease-in-out",
          flyoutOpen ? "translate-x-0" : "-translate-x-full",
        )}
      >
        <nav className="flex-1 overflow-y-auto py-2 px-2 space-y-0.5">
          {SERVICES.map((svc) => (
            <ServiceItem
              key={svc.id}
              service={svc}
              isActive={activeId === svc.id}
              isLoaded={loaded.has(svc.id)}
            />
          ))}
        </nav>

        <div className="px-3 py-3 border-t border-border-base">
          <p className="text-[10px] text-text-disabled tracking-widest uppercase">
            Ingwe
          </p>
        </div>
      </aside>
    </>
  );
}
