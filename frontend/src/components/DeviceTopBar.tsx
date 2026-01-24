import { useState, useEffect } from "preact/hooks";
import { Link } from "wouter-preact";
import { Bell, WifiOff } from "lucide-preact";
import { useWebSocket } from "../lib/websocket";
import { api, Printer } from "../lib/api";
import { useTheme } from "../lib/theme";

interface DeviceTopBarProps {
  selectedPrinter: string | null;
  onPrinterChange: (serial: string) => void;
}

export function DeviceTopBar({ selectedPrinter, onPrinterChange }: DeviceTopBarProps) {
  const [printers, setPrinters] = useState<Printer[]>([]);
  const [currentTime, setCurrentTime] = useState(new Date());
  const { deviceConnected } = useWebSocket();
  const { theme } = useTheme();

  useEffect(() => {
    loadPrinters();
    const timer = setInterval(() => setCurrentTime(new Date()), 1000);
    return () => clearInterval(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const loadPrinters = async () => {
    try {
      const data = await api.listPrinters();
      setPrinters(data);
      if (!selectedPrinter && data.length > 0) {
        onPrinterChange(data[0].serial);
      }
    } catch (e) {
      console.error("Failed to load printers:", e);
    }
  };

  const formatTime = (date: Date) => {
    return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  };

  // Get WiFi signal strength (mock for now, could come from WebSocket)
  const getWifiStrength = () => {
    return deviceConnected ? 4 : 0; // 0-4 levels
  };

  const wifiStrength = getWifiStrength();

  return (
    <div class="h-11 bg-black border-b border-bambu-dark-tertiary flex items-center px-3 gap-4">
      {/* Logo */}
      <Link href="/" class="flex-shrink-0">
        <img
          src={theme === "dark" ? "/spoolbuddy_logo_dark.png" : "/spoolbuddy_logo_light.png"}
          alt="SpoolBuddy"
          class="h-8"
        />
      </Link>

      {/* Printer selector - centered */}
      <div class="flex-1 flex justify-center">
        <select
          value={selectedPrinter || ""}
          onChange={(e) => onPrinterChange((e.target as HTMLSelectElement).value)}
          class="bg-bambu-dark-secondary text-white text-sm px-3 py-1.5 rounded border border-bambu-dark-tertiary focus:outline-none focus:border-bambu-green min-w-[150px]"
        >
          {printers.length === 0 ? (
            <option value="">No printers</option>
          ) : (
            printers.map((printer) => (
              <option key={printer.serial} value={printer.serial}>
                {printer.name}
              </option>
            ))
          )}
        </select>
      </div>

      {/* Right side indicators */}
      <div class="flex items-center gap-3">
        {/* WiFi signal */}
        <div class="flex items-center" title={deviceConnected ? "Connected" : "Disconnected"}>
          {deviceConnected ? (
            <div class="flex items-end gap-0.5 h-4">
              {[1, 2, 3, 4].map((level) => (
                <div
                  key={level}
                  class={`w-1 rounded-sm ${
                    level <= wifiStrength ? "bg-white" : "bg-bambu-dark-tertiary"
                  }`}
                  style={{ height: `${level * 4}px` }}
                />
              ))}
            </div>
          ) : (
            <WifiOff class="w-5 h-5 text-red-400" />
          )}
        </div>

        {/* Notification bell */}
        <button class="relative p-1 hover:bg-bambu-dark-secondary rounded transition-colors">
          <Bell class="w-5 h-5 text-white/70" />
          {/* Notification dot */}
          {/* <div class="absolute top-0 right-0 w-2 h-2 bg-red-500 rounded-full" /> */}
        </button>

        {/* Clock */}
        <span class="text-white/70 text-sm font-mono min-w-[50px] text-right">
          {formatTime(currentTime)}
        </span>
      </div>
    </div>
  );
}
