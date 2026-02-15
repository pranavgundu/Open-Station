import { useState, useEffect, useRef, useCallback } from "react";
import { RobotState } from "../../types";

interface Props {
  state: RobotState;
}

interface DataPoint {
  time: number;
  tripTime: number;
  lostPackets: number;
  voltage: number;
  cpu: number;
  mode: string;
  enabled: boolean;
}

const TIME_SCALES = [
  { label: "5s", ms: 5000 },
  { label: "30s", ms: 30000 },
  { label: "1m", ms: 60000 },
  { label: "5m", ms: 300000 },
];

export default function ChartsTab({ state }: Props) {
  const [scale, setScale] = useState(1); // index into TIME_SCALES
  const [data, setData] = useState<DataPoint[]>([]);
  const topCanvas = useRef<HTMLCanvasElement>(null);
  const bottomCanvas = useRef<HTMLCanvasElement>(null);

  // Collect data every second
  useEffect(() => {
    const interval = setInterval(() => {
      setData((prev) => {
        const point: DataPoint = {
          time: Date.now(),
          tripTime: state.trip_time_ms,
          lostPackets: state.lost_packets,
          voltage: state.voltage,
          cpu: 0, // placeholder
          mode: state.mode,
          enabled: state.enabled,
        };
        const cutoff = Date.now() - 300000; // keep 5 min max
        const next = [...prev.filter((p) => p.time > cutoff), point];
        return next;
      });
    }, 1000);
    return () => clearInterval(interval);
  }, [state]);

  const drawCharts = useCallback(() => {
    const timeWindow = TIME_SCALES[scale].ms;
    const now = Date.now();
    const visible = data.filter((p) => now - p.time <= timeWindow);

    // Draw top chart: trip time (green) + lost packets (orange)
    const topCtx = topCanvas.current?.getContext("2d");
    if (topCtx && topCanvas.current) {
      const w = topCanvas.current.width;
      const h = topCanvas.current.height;
      topCtx.clearRect(0, 0, w, h);
      topCtx.fillStyle = "#1a1a1a";
      topCtx.fillRect(0, 0, w, h);

      if (visible.length > 1) {
        // Trip time (green)
        topCtx.strokeStyle = "#00bc8c";
        topCtx.lineWidth = 1.5;
        topCtx.beginPath();
        visible.forEach((p, i) => {
          const x = ((p.time - (now - timeWindow)) / timeWindow) * w;
          const y = h - Math.min(p.tripTime / 100, 1) * h;
          i === 0 ? topCtx.moveTo(x, y) : topCtx.lineTo(x, y);
        });
        topCtx.stroke();

        // Lost packets (orange)
        topCtx.strokeStyle = "#f39c12";
        topCtx.lineWidth = 1.5;
        topCtx.beginPath();
        visible.forEach((p, i) => {
          const x = ((p.time - (now - timeWindow)) / timeWindow) * w;
          const y = h - Math.min(p.lostPackets / 50, 1) * h;
          i === 0 ? topCtx.moveTo(x, y) : topCtx.lineTo(x, y);
        });
        topCtx.stroke();
      }
    }

    // Draw bottom chart: voltage (yellow) + CPU (red) + mode bar
    const botCtx = bottomCanvas.current?.getContext("2d");
    if (botCtx && bottomCanvas.current) {
      const w = bottomCanvas.current.width;
      const h = bottomCanvas.current.height;
      botCtx.clearRect(0, 0, w, h);
      botCtx.fillStyle = "#1a1a1a";
      botCtx.fillRect(0, 0, w, h);

      if (visible.length > 1) {
        // Voltage (yellow)
        botCtx.strokeStyle = "#f1c40f";
        botCtx.lineWidth = 1.5;
        botCtx.beginPath();
        visible.forEach((p, i) => {
          const x = ((p.time - (now - timeWindow)) / timeWindow) * w;
          const y = h - 10 - (Math.min(p.voltage / 15, 1) * (h - 20));
          i === 0 ? botCtx.moveTo(x, y) : botCtx.lineTo(x, y);
        });
        botCtx.stroke();

        // Mode indicator bar at bottom
        visible.forEach((p) => {
          const x = ((p.time - (now - timeWindow)) / timeWindow) * w;
          botCtx.fillStyle = p.enabled
            ? p.mode === "Autonomous" ? "#3498db" : p.mode === "Test" ? "#9b59b6" : "#2ecc71"
            : "#555";
          botCtx.fillRect(x, h - 6, Math.max(w / visible.length, 2), 6);
        });
      }
    }
  }, [data, scale]);

  useEffect(() => {
    const frame = requestAnimationFrame(drawCharts);
    return () => cancelAnimationFrame(frame);
  }, [drawCharts]);

  return (
    <div className="flex flex-col h-full gap-2">
      <div className="flex items-center justify-between">
        <div className="text-xs text-gray-500 uppercase tracking-wider">Charts</div>
        <div className="flex gap-1">
          {TIME_SCALES.map((ts, i) => (
            <button
              key={ts.label}
              onClick={() => setScale(i)}
              className={`text-[10px] px-1.5 py-0.5 rounded ${
                scale === i ? "bg-blue-600 text-white" : "bg-[#2a2a2a] text-gray-400"
              }`}
            >
              {ts.label}
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 flex flex-col gap-1 min-h-0">
        <div className="flex-1 relative">
          <div className="absolute inset-0 flex items-center justify-between px-1 pointer-events-none">
            <span className="text-[9px] text-green-400/50">Trip Time</span>
            <span className="text-[9px] text-orange-400/50">Lost Pkts</span>
          </div>
          <canvas ref={topCanvas} width={260} height={80} className="w-full h-full rounded" />
        </div>
        <div className="flex-1 relative">
          <div className="absolute inset-0 flex items-center justify-between px-1 pointer-events-none">
            <span className="text-[9px] text-yellow-400/50">Voltage</span>
            <span className="text-[9px] text-red-400/50">CPU</span>
          </div>
          <canvas ref={bottomCanvas} width={260} height={80} className="w-full h-full rounded" />
        </div>
      </div>

      <div className="flex gap-3 text-[9px] text-gray-600">
        <span><span className="text-green-400">■</span> Trip Time</span>
        <span><span className="text-orange-400">■</span> Lost Pkts</span>
        <span><span className="text-yellow-400">■</span> Voltage</span>
      </div>
    </div>
  );
}
