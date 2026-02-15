import { RobotState } from "../../types";
import MessagesTab from "./MessagesTab";
import ChartsTab from "./ChartsTab";

interface Props {
  state: RobotState;
}

export default function BothTab({ state }: Props) {
  return (
    <div className="flex flex-col h-full gap-2">
      <div className="flex-1 min-h-0 overflow-hidden">
        <MessagesTab />
      </div>
      <div className="h-[180px] flex-shrink-0">
        <ChartsTab state={state} />
      </div>
    </div>
  );
}
