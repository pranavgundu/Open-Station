interface Tab {
  id: string;
  label: string;
  hasAlert?: boolean;
}

interface Props {
  tabs: Tab[];
  active: string;
  onSelect: (id: string) => void;
  position: "left" | "right";
}

export default function TabBar({ tabs, active, onSelect, position }: Props) {
  return (
    <div
      className={`flex flex-col bg-[#181818] ${
        position === "left" ? "border-r" : "border-l"
      } border-gray-700`}
    >
      {tabs.map((tab) => (
        <button
          key={tab.id}
          onClick={() => onSelect(tab.id)}
          className={`px-2 py-3 text-[10px] uppercase tracking-wider text-center w-12 transition-colors ${
            active === tab.id
              ? "bg-[#2d2d2d] text-white border-l-2 border-blue-500"
              : "text-gray-500 hover:text-gray-300 hover:bg-[#222]"
          } ${tab.hasAlert ? "!text-red-400" : ""}`}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}
