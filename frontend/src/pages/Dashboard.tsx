import { useEffect, useState, useMemo } from "preact/hooks";
import { Link } from "wouter-preact";
import { api, Spool, Printer, CloudAuthStatus } from "../lib/api";
import { useWebSocket } from "../lib/websocket";
import { useToast } from "../lib/toast";
import { Cloud, CloudOff, X, Download, Check, AlertTriangle, RefreshCw } from "lucide-preact";
import { SpoolIcon } from "../components/AmsCard";
import { AssignAmsModal } from "../components/AssignAmsModal";
import { LinkSpoolModal } from "../components/LinkSpoolModal";

// Storage keys for dashboard settings
const DEFAULT_CORE_WEIGHT_KEY = 'spoolbuddy-default-core-weight';

// Color palette for the cycling spool animation
const SPOOL_COLORS = [
  '#00AE42', // Green (Bambu)
  '#FF6B35', // Orange
  '#3B82F6', // Blue
  '#EF4444', // Red
  '#A855F7', // Purple
  '#FBBF24', // Yellow
  '#14B8A6', // Teal
  '#EC4899', // Pink
  '#F97316', // Amber
  '#22C55E', // Lime
];

// Empty state component with color-cycling spool
function ColorCyclingSpool() {
  const [colorIndex, setColorIndex] = useState(0);

  useEffect(() => {
    const interval = setInterval(() => {
      setColorIndex((prev) => (prev + 1) % SPOOL_COLORS.length);
    }, 2000);
    return () => clearInterval(interval);
  }, []);

  const currentColor = SPOOL_COLORS[colorIndex];

  return (
    <div class="flex flex-col items-center text-center">
      {/* Animated spool with NFC waves */}
      <div class="relative mb-6 flex items-center justify-center" style={{ width: 160, height: 160 }}>
        {/* NFC wave rings */}
        <div class="absolute w-24 h-24 rounded-full border-2 border-[var(--accent-color)]/30 animate-ping" style={{ animationDuration: '2.5s' }} />
        <div class="absolute w-32 h-32 rounded-full border border-[var(--accent-color)]/20 animate-ping" style={{ animationDuration: '2.5s', animationDelay: '0.4s' }} />
        <div class="absolute w-40 h-40 rounded-full border border-[var(--accent-color)]/10 animate-ping" style={{ animationDuration: '2.5s', animationDelay: '0.8s' }} />

        {/* Spool icon with color transition and glow */}
        <div class="relative">
          <div
            class="absolute -inset-4 rounded-full blur-2xl opacity-30 transition-colors duration-1000"
            style={{ backgroundColor: currentColor }}
          />
          <div class="transition-all duration-1000">
            <SpoolIcon color={currentColor} isEmpty={false} size={100} />
          </div>
        </div>
      </div>

      {/* Text content */}
      <div class="space-y-2">
        <p class="text-lg font-medium text-[var(--text-secondary)]">
          Ready to scan
        </p>
        <p class="text-sm text-[var(--text-muted)]">
          Place a spool on the scale to identify it
        </p>
      </div>

      {/* Subtle hint */}
      <div class="mt-6 flex items-center gap-2 text-xs text-[var(--text-muted)]/60">
        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <span>NFC tag will be read automatically</span>
      </div>
    </div>
  );
}

function getDefaultCoreWeight(): number {
  try {
    const stored = localStorage.getItem(DEFAULT_CORE_WEIGHT_KEY);
    if (stored) {
      const weight = parseInt(stored, 10);
      if (weight >= 0 && weight <= 500) return weight;
    }
  } catch {
    // Ignore errors
  }
  return 250; // Default 250g (typical Bambu spool core)
}

export function Dashboard() {
  const [spools, setSpools] = useState<Spool[]>([]);
  const [printers, setPrinters] = useState<Printer[]>([]);
  const [loading, setLoading] = useState(true);
  const { deviceConnected, deviceUpdateAvailable, currentWeight, weightStable, currentTagId, printerStatuses, printerStates } = useWebSocket();
  const [cloudStatus, setCloudStatus] = useState<CloudAuthStatus | null>(null);
  const [cloudBannerDismissed, setCloudBannerDismissed] = useState(() => {
    return localStorage.getItem('spoolbuddy-cloud-banner-dismissed') === 'true';
  });
  const [showAssignModal, setShowAssignModal] = useState(false);
  const [assignModalSpool, setAssignModalSpool] = useState<Spool | null>(null);
  const [showLinkModal, setShowLinkModal] = useState(false);
  const { showToast } = useToast();

  // Current Spool card state - persists until user closes or new tag detected
  const [displayedTagId, setDisplayedTagId] = useState<string | null>(null);
  const [displayedWeight, setDisplayedWeight] = useState<number | null>(null);
  const [cardDismissed, setCardDismissed] = useState(false);

  // Compute spools without tags for the Link To Spool feature
  const untaggedSpools = useMemo(() => {
    return spools.filter(s => !s.tag_id && !s.archived_at);
  }, [spools]);

  // Find spool by tag_id in the loaded spools list
  const findSpoolByTagId = (tagId: string | null, spoolList: Spool[]): Spool | null => {
    if (!tagId) return null;
    return spoolList.find(s => s.tag_id === tagId) || null;
  };

  useEffect(() => {
    loadSpools();
    loadPrinters();
    loadCloudStatus();
  }, []);

  // Compute displayed spool from displayedTagId
  const displayedSpool = displayedTagId ? findSpoolByTagId(displayedTagId, spools) : null;

  // Handle tag detection - show card when tag detected, keep until user closes or new tag
  useEffect(() => {
    if (currentTagId) {
      // New tag detected - check if it's different from current display
      if (currentTagId !== displayedTagId) {
        // Show the new tag and reset dismissed state
        setDisplayedTagId(currentTagId);
        setDisplayedWeight(null);
        setCardDismissed(false);
      }
      // Update weight when stable
      if (currentWeight !== null && weightStable) {
        setDisplayedWeight(Math.round(Math.max(0, currentWeight)));
      }
    }
    // Note: We don't clear displayedTagId when currentTagId becomes null
    // The card persists until user clicks Close or a new tag is detected
  }, [currentTagId, currentWeight, weightStable, displayedTagId]);

  // Close handler for the Current Spool card
  const handleCloseSpoolCard = () => {
    setCardDismissed(true);
    setDisplayedTagId(null);
    setDisplayedWeight(null);
  };

  // Link tag to an existing spool
  const handleLinkTagToSpool = async (spool: Spool) => {
    if (!displayedTagId) return;

    try {
      await api.updateSpool(spool.id, { ...spool, tag_id: displayedTagId });
      showToast('success', `Linked tag to ${spool.brand || 'Unknown'} ${spool.material}`);
      setShowLinkModal(false);
      // Refresh spools to show updated data and the card will now show as known spool
      loadSpools();
    } catch (e) {
      console.error('Failed to link tag:', e);
      showToast('error', 'Failed to link tag to spool');
    }
  };

  // Sync weight: trust scale weight and reset tracking
  const handleSyncWeight = async (spool: Spool) => {
    // Use live scale weight, not stored weight
    const weightToSync = currentWeight !== null ? Math.round(Math.max(0, currentWeight)) : spool.weight_current;
    if (weightToSync === null) return;
    try {
      await api.setSpoolWeight(spool.id, weightToSync);
      // Reset weight tracking to allow fresh comparison
      setWeightUpdatedForSpool(null);
      // Update displayed weight to match synced weight
      setDisplayedWeight(weightToSync);
      await loadSpools();
      showToast('success', `Synced weight for ${spool.brand || ''} ${spool.material}`);
    } catch (e) {
      showToast('error', e instanceof Error ? e.message : 'Failed to sync weight');
    }
  };

  // Track if we've updated weight for current spool to avoid duplicate updates
  const [weightUpdatedForSpool, setWeightUpdatedForSpool] = useState<string | null>(null);

  // Reset weight tracking when spool changes
  useEffect(() => {
    if (displayedSpool?.id !== weightUpdatedForSpool) {
      setWeightUpdatedForSpool(null);
    }
  }, [displayedSpool?.id]);

  // Update spool weight in backend when known spool is detected on scale
  useEffect(() => {
    // Only update weight once per spool detection, when weight is stable and tag is currently on scale
    if (displayedSpool && currentTagId && currentWeight !== null && weightStable && weightUpdatedForSpool !== displayedSpool.id) {
      const newWeight = Math.round(Math.max(0, currentWeight));
      // Only update if weight is different (any change counts)
      if (displayedSpool.weight_current === null || displayedSpool.weight_current !== newWeight) {
        // Use setSpoolWeight to properly sync and reset consumed_since_weight
        api.setSpoolWeight(displayedSpool.id, newWeight)
          .then(() => {
            // Mark as updated to prevent duplicate updates
            setWeightUpdatedForSpool(displayedSpool.id);
            // Refresh spools list to keep it in sync
            loadSpools();
          })
          .catch(err => console.error('Failed to update spool weight:', err));
      } else {
        // Weight is same, just mark as processed
        setWeightUpdatedForSpool(displayedSpool.id);
      }
    }
  }, [displayedSpool?.id, currentTagId, currentWeight, weightStable, weightUpdatedForSpool]);

  const loadCloudStatus = async () => {
    try {
      const status = await api.getCloudStatus();
      setCloudStatus(status);
    } catch (e) {
      console.error("Failed to load cloud status:", e);
    }
  };

  const dismissCloudBanner = () => {
    setCloudBannerDismissed(true);
    localStorage.setItem('spoolbuddy-cloud-banner-dismissed', 'true');
  };

  const loadSpools = async () => {
    try {
      const data = await api.listSpools();
      setSpools(data);
    } catch (e) {
      console.error("Failed to load spools:", e);
    } finally {
      setLoading(false);
    }
  };

  const loadPrinters = async () => {
    try {
      const data = await api.listPrinters();
      setPrinters(data);
    } catch (e) {
      console.error("Failed to load printers:", e);
    }
  };

  // Get effective connection status for a printer
  const isPrinterConnected = (printer: Printer) => {
    return printerStatuses.get(printer.serial) ?? printer.connected ?? false;
  };

  // Get printer state info for display
  const getPrinterStateInfo = (printer: Printer) => {
    const connected = isPrinterConnected(printer);
    if (!connected) {
      return { status: "Offline", color: "text-[var(--text-muted)]", bgColor: "bg-[var(--bg-secondary)]" };
    }

    const state = printerStates.get(printer.serial);
    if (!state || !state.gcode_state) {
      return { status: "Connected", color: "text-green-500", bgColor: "bg-green-500/20" };
    }

    const gcodeState = state.gcode_state.toUpperCase();
    switch (gcodeState) {
      case "RUNNING":
        return {
          status: "Printing",
          color: "text-blue-400",
          bgColor: "bg-blue-500/20",
          progress: state.print_progress,
          jobName: state.subtask_name,
          remainingTime: state.mc_remaining_time
        };
      case "PAUSE":
        return {
          status: "Paused",
          color: "text-yellow-500",
          bgColor: "bg-yellow-500/20",
          progress: state.print_progress,
          jobName: state.subtask_name
        };
      case "FINISH":
        return { status: "Finished", color: "text-green-500", bgColor: "bg-green-500/20" };
      case "FAILED":
        return { status: "Failed", color: "text-red-500", bgColor: "bg-red-500/20" };
      case "PREPARE":
        return { status: "Preparing", color: "text-cyan-400", bgColor: "bg-cyan-500/20" };
      case "SLICING":
        return { status: "Slicing", color: "text-purple-400", bgColor: "bg-purple-500/20" };
      case "IDLE":
      default:
        return { status: "Idle", color: "text-green-500", bgColor: "bg-green-500/20" };
    }
  };

  // Calculate stats
  const totalSpools = spools.length;
  const materials = new Set(spools.map((s) => s.material)).size;
  const brands = new Set(spools.filter((s) => s.brand).map((s) => s.brand)).size;

  return (
    <div class="space-y-8">
      {/* Page header with stats bar */}
      <div class="flex flex-col gap-4">
        <div class="flex items-center justify-between">
          <h1 class="text-2xl font-bold text-[var(--text-primary)]">Dashboard</h1>
          <div class="flex items-center gap-3">
            {/* Cloud status indicator */}
            {cloudStatus && (
              <Link
                href="/settings"
                class={`flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium transition-all hover:scale-105 ${
                  cloudStatus.is_authenticated
                    ? "bg-green-500/15 text-green-500 hover:bg-green-500/25"
                    : "bg-[var(--bg-tertiary)] text-[var(--text-muted)] hover:bg-[var(--border-color)]"
                }`}
                title={cloudStatus.is_authenticated ? `Cloud: ${cloudStatus.email}` : "Cloud: Not connected"}
              >
                {cloudStatus.is_authenticated ? (
                  <Cloud class="w-3.5 h-3.5" />
                ) : (
                  <CloudOff class="w-3.5 h-3.5" />
                )}
                <span class="hidden sm:inline">
                  {cloudStatus.is_authenticated ? "Cloud" : "Offline"}
                </span>
              </Link>
            )}
            {/* Add spool button */}
            <Link href="/inventory?add=true" class="btn btn-primary btn-sm">
              <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
              </svg>
              <span class="hidden sm:inline">Add Spool</span>
            </Link>
          </div>
        </div>

        {/* Compact stats bar */}
        <div class="flex items-center gap-6 px-4 py-3 bg-[var(--bg-secondary)] rounded-xl border border-[var(--border-color)]">
          <div class="flex items-center gap-2">
            <span class="text-2xl font-bold text-[var(--text-primary)]">{loading ? "—" : totalSpools}</span>
            <span class="text-sm text-[var(--text-muted)]">Spools</span>
          </div>
          <div class="w-px h-6 bg-[var(--border-color)]" />
          <div class="flex items-center gap-2">
            <span class="text-2xl font-bold text-[var(--text-primary)]">{loading ? "—" : materials}</span>
            <span class="text-sm text-[var(--text-muted)]">Materials</span>
          </div>
          <div class="w-px h-6 bg-[var(--border-color)]" />
          <div class="flex items-center gap-2">
            <span class="text-2xl font-bold text-[var(--text-primary)]">{loading ? "—" : brands}</span>
            <span class="text-sm text-[var(--text-muted)]">Brands</span>
          </div>
        </div>
      </div>

      {/* Cloud status banner */}
      {cloudStatus && !cloudStatus.is_authenticated && !cloudBannerDismissed && (
        <div class="flex items-center justify-between gap-4 p-4 bg-[var(--accent-color)]/10 border border-[var(--accent-color)]/20 rounded-xl">
          <div class="flex items-center gap-3">
            <div class="p-2 bg-[var(--accent-color)]/20 rounded-lg">
              <CloudOff class="w-5 h-5 text-[var(--accent-color)]" />
            </div>
            <div>
              <p class="text-sm font-medium text-[var(--text-primary)]">
                Connect to Bambu Cloud
              </p>
              <p class="text-xs text-[var(--text-secondary)]">
                Access your custom slicer presets
              </p>
            </div>
          </div>
          <div class="flex items-center gap-2">
            <Link href="/settings" class="btn btn-primary btn-sm">
              Connect
            </Link>
            <button
              onClick={dismissCloudBanner}
              class="p-1.5 text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors rounded-lg hover:bg-[var(--bg-tertiary)]"
              title="Dismiss"
            >
              <X class="w-4 h-4" />
            </button>
          </div>
        </div>
      )}

      {/* Main content grid - Current Spool as hero + side panels */}
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">

        {/* Left column: Device Status + Printers */}
        <div class="space-y-6 lg:col-span-1">

          {/* Device status - compact visual */}
          <div class="card p-5">
            <div class="flex items-center justify-between mb-4">
              <h2 class="text-sm font-semibold text-[var(--text-primary)] uppercase tracking-wide">Device</h2>
              {deviceUpdateAvailable && (
                <Link
                  href="/settings#updates"
                  class="flex items-center gap-1.5 px-2 py-1 rounded-md text-xs font-medium bg-blue-500/20 text-blue-400 hover:bg-blue-500/30 transition-colors"
                >
                  <Download class="w-3 h-3" />
                  Update
                </Link>
              )}
            </div>

            <div class="space-y-3">
              {/* Connection status */}
              <div class="flex items-center gap-3">
                <div class={`w-2.5 h-2.5 rounded-full ${deviceConnected ? 'bg-green-500 animate-pulse' : 'bg-red-500'}`} />
                <span class="text-sm text-[var(--text-secondary)]">
                  {deviceConnected ? "Connected" : "Disconnected"}
                </span>
              </div>

              {/* Scale weight */}
              <div class="flex items-center justify-between p-3 bg-[var(--bg-tertiary)] rounded-lg">
                <div class="flex items-center gap-2">
                  <svg class="w-4 h-4 text-[var(--text-muted)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 6l3 1m0 0l-3 9a5.002 5.002 0 006.001 0M6 7l3 9M6 7l6-2m6 2l3-1m-3 1l-3 9a5.002 5.002 0 006.001 0M18 7l3 9m-3-9l-6-2m0-2v2m0 16V5m0 16H9m3 0h3" />
                  </svg>
                  <span class="text-xs text-[var(--text-muted)]">Scale</span>
                </div>
                <div class="flex items-center gap-2">
                  <span class="text-lg font-mono font-semibold text-[var(--text-primary)]">
                    {currentWeight !== null ? `${Math.abs(currentWeight) <= 20 ? 0 : Math.round(Math.max(0, currentWeight))}g` : '—'}
                  </span>
                  {weightStable && currentWeight !== null && (
                    <span class="w-2 h-2 rounded-full bg-green-500" title="Stable" />
                  )}
                </div>
              </div>

              {/* NFC status */}
              <div class="flex items-center justify-between p-3 bg-[var(--bg-tertiary)] rounded-lg">
                <div class="flex items-center gap-2">
                  <svg class="w-4 h-4 text-[var(--text-muted)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A2 2 0 013 12V7a4 4 0 014-4z" />
                  </svg>
                  <span class="text-xs text-[var(--text-muted)]">NFC</span>
                </div>
                <span class={`text-sm font-medium ${currentTagId ? 'text-green-500' : 'text-[var(--text-muted)]'}`}>
                  {currentTagId ? 'Tag detected' : 'No tag'}
                </span>
              </div>
            </div>
          </div>

          {/* Printers - compact list */}
          {printers.length > 0 && (
            <div class="card p-5">
              <div class="flex items-center justify-between mb-4">
                <h2 class="text-sm font-semibold text-[var(--text-primary)] uppercase tracking-wide">Printers</h2>
                <Link href="/printers" class="text-xs text-[var(--accent-color)] hover:underline">
                  View all
                </Link>
              </div>
              <div class="space-y-2">
                {printers.map((printer) => {
                  const stateInfo = getPrinterStateInfo(printer);
                  const hasProgress = stateInfo.progress !== undefined && stateInfo.progress !== null;

                  // Status color for left border
                  const borderColor = stateInfo.status === "Printing" ? "border-l-blue-500"
                    : stateInfo.status === "Paused" ? "border-l-yellow-500"
                    : stateInfo.status === "Idle" ? "border-l-green-500"
                    : stateInfo.status === "Offline" ? "border-l-gray-500"
                    : "border-l-green-500";

                  return (
                    <Link
                      key={printer.serial}
                      href="/printers"
                      class={`block p-3 bg-[var(--bg-tertiary)] rounded-lg border-l-3 ${borderColor} hover:bg-[var(--border-color)] transition-all hover:translate-x-0.5`}
                    >
                      <div class="flex items-center justify-between gap-2">
                        <div class="min-w-0 flex-1">
                          <p class="text-sm font-medium text-[var(--text-primary)] truncate">{printer.name || printer.serial}</p>
                          {hasProgress ? (
                            <div class="flex items-center gap-2 mt-1">
                              <div class="flex-1 h-1 bg-[var(--bg-secondary)] rounded-full overflow-hidden">
                                <div
                                  class={`h-full rounded-full ${stateInfo.status === "Paused" ? "bg-yellow-500" : "bg-blue-500"}`}
                                  style={{ width: `${stateInfo.progress}%` }}
                                />
                              </div>
                              <span class="text-xs text-[var(--text-muted)]">{stateInfo.progress}%</span>
                            </div>
                          ) : (
                            <p class="text-xs text-[var(--text-muted)]">{stateInfo.status}</p>
                          )}
                        </div>
                      </div>
                    </Link>
                  );
                })}
              </div>
            </div>
          )}
        </div>

        {/* Right column: Current Spool - HERO */}
        <div class="lg:col-span-2">
          <div class="card p-6 h-full min-h-[400px] flex flex-col">
            <div class="flex items-center justify-between mb-4">
              <h2 class="text-sm font-semibold text-[var(--text-primary)] uppercase tracking-wide">Current Spool</h2>
            </div>
            <div class="flex-1 flex items-center justify-center">

            {displayedTagId && !cardDismissed ? (
              displayedSpool ? (
                // Known spool from inventory
                (() => {
                  // Always use live scale weight when tag is on scale, fall back to stored weight_current
                  const isTagOnScale = currentTagId === displayedTagId && currentWeight !== null;
                  const liveWeight = isTagOnScale ? Math.round(Math.max(0, currentWeight)) : null;
                  const storedWeight = displayedSpool.weight_current;
                  const grossWeight = liveWeight ?? storedWeight ?? displayedWeight;
                  // Always use Default Core Weight from settings for dashboard calculation
                  const coreWeight = getDefaultCoreWeight();
                  const remaining = grossWeight !== null
                    ? Math.round(Math.max(0, grossWeight - coreWeight))
                    : null;
                  const labelWeight = Math.round(displayedSpool.label_weight || 1000);
                  const fillPercent = remaining !== null ? Math.min(100, Math.round((remaining / labelWeight) * 100)) : null;
                  const fillColor = fillPercent !== null
                    ? fillPercent > 50 ? '#22c55e' : fillPercent > 20 ? '#eab308' : '#ef4444'
                    : '#808080';

                  return (
                    <div class="flex flex-col items-center space-y-4 max-w-md">
                      {/* Top section: Spool icon + main info - centered */}
                      <div class="flex items-start gap-5">
                        {/* Spool visualization */}
                        <div class="relative flex-shrink-0">
                          <SpoolIcon
                            color={displayedSpool.rgba ? `#${displayedSpool.rgba.slice(0, 6)}` : '#808080'}
                            isEmpty={false}
                            size={100}
                          />
                          {fillPercent !== null && (
                            <div
                              class="absolute -bottom-2 -right-2 px-2 py-0.5 rounded-full text-xs font-bold text-white shadow-lg"
                              style={{ backgroundColor: fillColor }}
                            >
                              {fillPercent}%
                            </div>
                          )}
                        </div>

                        {/* Main info */}
                        <div class="flex-1 min-w-0 pt-1">
                          <h3 class="text-lg font-semibold text-[var(--text-primary)]">
                            {displayedSpool.color_name || "Unknown color"}
                          </h3>
                          <p class="text-sm text-[var(--text-secondary)]">
                            {displayedSpool.brand} • {displayedSpool.material}
                            {displayedSpool.subtype && ` ${displayedSpool.subtype}`}
                          </p>

                          {/* Filament remaining - big number */}
                          {remaining !== null && (
                            <div class="mt-3">
                              <div class="flex items-baseline gap-2">
                                <span class="text-3xl font-bold font-mono text-[var(--text-primary)]">{remaining}g</span>
                                <span class="text-sm text-[var(--text-muted)]">/ {labelWeight}g</span>
                              </div>
                              <p class="text-xs text-[var(--text-muted)] mt-0.5">Filament remaining</p>

                              {/* Fill bar */}
                              <div class="mt-2 max-w-xs">
                                <div class="h-2 bg-[var(--bg-tertiary)] rounded-full overflow-hidden">
                                  <div
                                    class="h-full rounded-full transition-all duration-500"
                                    style={{ width: `${fillPercent}%`, backgroundColor: fillColor }}
                                  />
                                </div>
                              </div>
                            </div>
                          )}
                        </div>
                      </div>

                      {/* Details grid */}
                      {(() => {
                        // Use live weight from outer scope, or fall back to stored weight
                        const scaleWeightDisplay = liveWeight ?? displayedSpool.weight_current;
                        // Calculate expected gross weight using Default Core Weight (same as remaining calculation)
                        // net_weight = label_weight - weight_used - consumed_since_weight
                        const netWeight = Math.max(0,
                          (displayedSpool.label_weight || 0) -
                          (displayedSpool.weight_used || 0) -
                          (displayedSpool.consumed_since_weight || 0)
                        );
                        const calculatedWeight = netWeight + coreWeight; // Uses default core weight from outer scope
                        // Recalculate difference and match based on live weight
                        const difference = scaleWeightDisplay !== null ? scaleWeightDisplay - calculatedWeight : null;
                        const isMatch = difference !== null ? Math.abs(difference) <= 50 : null;
                        const diffStr = difference !== null ? (difference > 0 ? `+${Math.round(difference)}` : Math.round(difference)) : '?';
                        const scaleTooltip = isMatch
                          ? `Scale: ${scaleWeightDisplay !== null ? Math.round(scaleWeightDisplay) : '—'}g\nCalculated: ${Math.round(calculatedWeight)}g\nDifference: ${diffStr}g (within tolerance)`
                          : `Scale: ${scaleWeightDisplay !== null ? Math.round(scaleWeightDisplay) : '—'}g\nCalculated: ${Math.round(calculatedWeight)}g\nDifference: ${diffStr}g (mismatch!)`;

                        return (
                          <div class="grid grid-cols-2 gap-x-6 gap-y-2 text-sm bg-[var(--bg-tertiary)] rounded-lg p-3">
                            <div class="flex justify-between">
                              <span class="text-[var(--text-muted)]">Gross weight</span>
                              <span class="font-mono text-[var(--text-primary)]">{grossWeight !== null ? `${grossWeight}g` : '—'}</span>
                            </div>
                            <div class="flex justify-between">
                              <span class="text-[var(--text-muted)]">Core weight</span>
                              <span class="font-mono text-[var(--text-primary)]">{coreWeight}g</span>
                            </div>
                            <div class="flex justify-between">
                              <span class="text-[var(--text-muted)]">Spool size</span>
                              <span class="font-mono text-[var(--text-primary)]">{labelWeight}g</span>
                            </div>
                            <div class="flex justify-between items-center" title={scaleTooltip}>
                              <span class="text-[var(--text-muted)]">Scale</span>
                              {scaleWeightDisplay !== null ? (
                                <span class={`flex items-center gap-1 font-mono ${isMatch ? 'text-green-500' : 'text-yellow-500'}`}>
                                  {Math.round(scaleWeightDisplay)}g
                                  {isMatch ? (
                                    <Check class="w-3 h-3" />
                                  ) : (
                                    <>
                                      <AlertTriangle class="w-3 h-3" />
                                      <button
                                        onClick={() => handleSyncWeight(displayedSpool)}
                                        class="p-1 hover:bg-[var(--accent-color)]/20 rounded transition-colors text-[var(--accent-color)]"
                                        title="Sync: trust scale weight and reset tracking"
                                      >
                                        <RefreshCw class="w-3.5 h-3.5" />
                                      </button>
                                    </>
                                  )}
                                </span>
                              ) : (
                                <span class="text-[var(--text-muted)]">—</span>
                              )}
                            </div>
                            <div class="flex justify-between items-center">
                              <span class="text-[var(--text-muted)]">K Profile</span>
                              {displayedSpool.ext_has_k ? (
                                <span class="flex items-center gap-1 text-green-500">
                                  <Check class="w-3 h-3" />
                                  Yes
                                </span>
                              ) : (
                                <span class="text-[var(--text-muted)]">No</span>
                              )}
                            </div>
                            <div class="flex justify-between items-center">
                              <span class="text-[var(--text-muted)]">Tag</span>
                              <span class="font-mono text-xs text-[var(--text-secondary)] truncate max-w-[120px]" title={displayedTagId || ''}>
                                {displayedTagId ? displayedTagId.slice(-8) : '—'}
                              </span>
                            </div>
                            {displayedSpool.location && (
                              <div class="flex justify-between col-span-2">
                                <span class="text-[var(--text-muted)]">Location</span>
                                <span class="text-[var(--text-primary)]">{displayedSpool.location}</span>
                              </div>
                            )}
                          </div>
                        );
                      })()}

                      {/* Action buttons */}
                      <div class="flex gap-2 justify-center">
                        <button
                          onClick={() => {
                            setAssignModalSpool(displayedSpool);
                            setShowAssignModal(true);
                          }}
                          class="btn btn-primary"
                        >
                          Configure AMS
                        </button>
                        <Link
                          href={`/inventory?edit=${encodeURIComponent(displayedSpool.id)}`}
                          class="btn btn-primary"
                        >
                          Edit Spool
                        </Link>
                        <button
                          onClick={handleCloseSpoolCard}
                          class="btn btn-secondary"
                        >
                          Close
                        </button>
                      </div>
                    </div>
                  );
                })()
              ) : (
                // Unknown tag - not in inventory
                (() => {
                  const defaultCoreWeight = getDefaultCoreWeight();
                  // Prefer live weight when tag is on scale, fall back to stored weight when removed
                  const grossWeight = (currentTagId === displayedTagId && currentWeight !== null)
                    ? Math.round(Math.max(0, currentWeight))
                    : displayedWeight;
                  const estimatedRemaining = grossWeight !== null
                    ? Math.round(Math.max(0, grossWeight - defaultCoreWeight))
                    : null;

                  return (
                    <div class="flex flex-col items-center text-center space-y-5">
                      <div class="w-20 h-20 rounded-2xl bg-[var(--accent-color)]/15 flex items-center justify-center">
                        <svg class="w-10 h-10 text-[var(--accent-color)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A2 2 0 013 12V7a4 4 0 014-4z" />
                        </svg>
                      </div>
                      <div>
                        <h3 class="text-lg font-semibold text-[var(--text-primary)]">New Tag Detected</h3>
                        <p class="text-sm text-[var(--text-muted)] font-mono mt-1">{displayedTagId}</p>
                      </div>
                      {grossWeight !== null && (
                        <div class="text-sm text-[var(--text-secondary)]">
                          <span class="font-mono font-semibold">{grossWeight}g</span> on scale
                          {estimatedRemaining !== null && estimatedRemaining > 0 && (
                            <span class="text-[var(--text-muted)]"> • ~{estimatedRemaining}g filament</span>
                          )}
                        </div>
                      )}
                      <div class="flex flex-wrap gap-2 justify-center">
                        <Link
                          href={`/inventory?add=true&tagId=${encodeURIComponent(displayedTagId)}${grossWeight !== null ? `&weight=${grossWeight}` : ''}`}
                          class="btn btn-primary"
                        >
                          <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
                          </svg>
                          Add Spool
                        </Link>
                        <button
                          onClick={() => setShowLinkModal(true)}
                          disabled={untaggedSpools.length === 0}
                          class={`btn ${untaggedSpools.length > 0 ? 'btn-primary' : 'btn-secondary'}`}
                          title={untaggedSpools.length === 0 ? 'No spools without tags in inventory' : ''}
                        >
                          <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
                          </svg>
                          Link To Spool
                        </button>
                        <button
                          onClick={handleCloseSpoolCard}
                          class="btn btn-secondary"
                        >
                          Close
                        </button>
                      </div>
                    </div>
                  );
                })()
              )
            ) : (
              // No tag - empty state with color-cycling spool
              <ColorCyclingSpool />
            )}
            </div>
          </div>
        </div>
      </div>

      {/* Assign to AMS Modal */}
      {assignModalSpool && (
        <AssignAmsModal
          isOpen={showAssignModal}
          onClose={() => {
            setShowAssignModal(false);
            setAssignModalSpool(null);
          }}
          spool={assignModalSpool}
        />
      )}

      {/* Link Tag to Spool Modal */}
      {displayedTagId && (
        <LinkSpoolModal
          isOpen={showLinkModal}
          onClose={() => setShowLinkModal(false)}
          tagId={displayedTagId}
          untaggedSpools={untaggedSpools}
          onLink={handleLinkTagToSpool}
        />
      )}
    </div>
  );
}
