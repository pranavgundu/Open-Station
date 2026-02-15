import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

export function useTauriEvent<T>(event: string, initial: T): T {
  const [value, setValue] = useState<T>(initial);
  useEffect(() => {
    const unlisten = listen<T>(event, (e) => setValue(e.payload));
    return () => {
      unlisten.then((f) => f());
    };
  }, [event]);
  return value;
}
