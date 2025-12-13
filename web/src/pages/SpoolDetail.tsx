import { useEffect, useState } from "preact/hooks";
import { useRoute, useLocation } from "wouter-preact";
import { api, Spool } from "../lib/api";

export function SpoolDetail() {
  const [, params] = useRoute("/spool/:id");
  const [, navigate] = useLocation();
  const [spool, setSpool] = useState<Spool | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);

  // Form state
  const [material, setMaterial] = useState("");
  const [subtype, setSubtype] = useState("");
  const [colorName, setColorName] = useState("");
  const [rgba, setRgba] = useState("");
  const [brand, setBrand] = useState("");
  const [labelWeight, setLabelWeight] = useState("");
  const [coreWeight, setCoreWeight] = useState("");
  const [slicerFilament, setSlicerFilament] = useState("");
  const [note, setNote] = useState("");

  useEffect(() => {
    if (params?.id) {
      loadSpool(params.id);
    }
  }, [params?.id]);

  const loadSpool = async (id: string) => {
    try {
      const data = await api.getSpool(id);
      setSpool(data);
      // Populate form
      setMaterial(data.material);
      setSubtype(data.subtype || "");
      setColorName(data.color_name || "");
      setRgba(data.rgba || "");
      setBrand(data.brand || "");
      setLabelWeight(data.label_weight?.toString() || "");
      setCoreWeight(data.core_weight?.toString() || "");
      setSlicerFilament(data.slicer_filament || "");
      setNote(data.note || "");
    } catch (e) {
      console.error("Failed to load spool:", e);
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async (e: Event) => {
    e.preventDefault();
    if (!spool) return;

    setSaving(true);
    try {
      await api.updateSpool(spool.id, {
        material,
        subtype: subtype || null,
        color_name: colorName || null,
        rgba: rgba || null,
        brand: brand || null,
        label_weight: labelWeight ? parseInt(labelWeight) : null,
        core_weight: coreWeight ? parseInt(coreWeight) : null,
        slicer_filament: slicerFilament || null,
        note: note || null,
        tag_id: spool.tag_id,
        data_origin: spool.data_origin,
        tag_type: spool.tag_type,
      });
      alert("Saved!");
    } catch (e) {
      console.error("Failed to save:", e);
      alert("Failed to save");
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    if (!spool) return;
    if (!confirm("Delete this spool?")) return;

    setDeleting(true);
    try {
      await api.deleteSpool(spool.id);
      navigate("/inventory");
    } catch (e) {
      console.error("Failed to delete:", e);
      alert("Failed to delete");
      setDeleting(false);
    }
  };

  if (loading) {
    return (
      <div class="p-8 text-center text-gray-500">Loading...</div>
    );
  }

  if (!spool) {
    return (
      <div class="p-8 text-center text-gray-500">Spool not found</div>
    );
  }

  const colorHex = rgba ? `#${rgba.slice(0, 6)}` : "#cccccc";

  return (
    <div class="space-y-6">
      {/* Header */}
      <div class="flex items-center space-x-4">
        <button
          onClick={() => navigate("/inventory")}
          class="p-2 text-gray-400 hover:text-gray-600"
        >
          <svg class="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18" />
          </svg>
        </button>
        <div
          class="w-12 h-12 rounded-full border-2 border-gray-200"
          style={{ backgroundColor: colorHex }}
        />
        <div>
          <h1 class="text-2xl font-bold text-gray-900">
            {colorName || "Unnamed Spool"}
          </h1>
          <p class="text-gray-600">
            {brand} {material} {subtype}
          </p>
        </div>
      </div>

      {/* Form */}
      <form onSubmit={handleSave} class="bg-white rounded-lg shadow">
        <div class="p-6 space-y-6">
          {/* Basic info */}
          <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <label class="block text-sm font-medium text-gray-700">
                Material *
              </label>
              <select
                value={material}
                onChange={(e) => setMaterial((e.target as HTMLSelectElement).value)}
                class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-primary-500 focus:border-primary-500"
              >
                <option>PLA</option>
                <option>PETG</option>
                <option>ABS</option>
                <option>ASA</option>
                <option>TPU</option>
                <option>PA</option>
                <option>PC</option>
              </select>
            </div>
            <div>
              <label class="block text-sm font-medium text-gray-700">
                Subtype
              </label>
              <input
                type="text"
                value={subtype}
                onInput={(e) => setSubtype((e.target as HTMLInputElement).value)}
                placeholder="e.g., Basic, Silk, Matte"
                class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-primary-500 focus:border-primary-500"
              />
            </div>
          </div>

          {/* Color */}
          <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <label class="block text-sm font-medium text-gray-700">
                Color Name
              </label>
              <input
                type="text"
                value={colorName}
                onInput={(e) => setColorName((e.target as HTMLInputElement).value)}
                placeholder="e.g., Red, Galaxy Black"
                class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-primary-500 focus:border-primary-500"
              />
            </div>
            <div>
              <label class="block text-sm font-medium text-gray-700">
                RGBA Color Code
              </label>
              <div class="mt-1 flex items-center space-x-2">
                <input
                  type="text"
                  value={rgba}
                  onInput={(e) => setRgba((e.target as HTMLInputElement).value.toUpperCase())}
                  placeholder="RRGGBBAA"
                  maxLength={8}
                  class="block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-primary-500 focus:border-primary-500 font-mono"
                />
                <div
                  class="w-10 h-10 rounded border border-gray-300"
                  style={{ backgroundColor: colorHex }}
                />
              </div>
            </div>
          </div>

          {/* Brand and slicer */}
          <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <label class="block text-sm font-medium text-gray-700">Brand</label>
              <input
                type="text"
                value={brand}
                onInput={(e) => setBrand((e.target as HTMLInputElement).value)}
                placeholder="e.g., Bambu, Prusament"
                class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-primary-500 focus:border-primary-500"
              />
            </div>
            <div>
              <label class="block text-sm font-medium text-gray-700">
                Slicer Filament ID
              </label>
              <input
                type="text"
                value={slicerFilament}
                onInput={(e) => setSlicerFilament((e.target as HTMLInputElement).value)}
                placeholder="e.g., GFA00"
                class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-primary-500 focus:border-primary-500 font-mono"
              />
            </div>
          </div>

          {/* Weights */}
          <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <label class="block text-sm font-medium text-gray-700">
                Label Weight (g)
              </label>
              <input
                type="number"
                value={labelWeight}
                onInput={(e) => setLabelWeight((e.target as HTMLInputElement).value)}
                placeholder="1000"
                class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-primary-500 focus:border-primary-500"
              />
            </div>
            <div>
              <label class="block text-sm font-medium text-gray-700">
                Core Weight (g)
              </label>
              <input
                type="number"
                value={coreWeight}
                onInput={(e) => setCoreWeight((e.target as HTMLInputElement).value)}
                placeholder="250"
                class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-primary-500 focus:border-primary-500"
              />
            </div>
          </div>

          {/* Note */}
          <div>
            <label class="block text-sm font-medium text-gray-700">Note</label>
            <textarea
              value={note}
              onInput={(e) => setNote((e.target as HTMLTextAreaElement).value)}
              rows={3}
              class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-primary-500 focus:border-primary-500"
            />
          </div>

          {/* Tag info (read-only) */}
          {spool.tag_id && (
            <div class="bg-gray-50 rounded-lg p-4">
              <h3 class="text-sm font-medium text-gray-700 mb-2">Tag Information</h3>
              <dl class="grid grid-cols-2 gap-2 text-sm">
                <dt class="text-gray-500">Tag ID</dt>
                <dd class="font-mono">{spool.tag_id}</dd>
                <dt class="text-gray-500">Tag Type</dt>
                <dd>{spool.tag_type || "Unknown"}</dd>
                <dt class="text-gray-500">Data Origin</dt>
                <dd>{spool.data_origin || "Unknown"}</dd>
              </dl>
            </div>
          )}
        </div>

        {/* Actions */}
        <div class="px-6 py-4 bg-gray-50 border-t border-gray-200 flex justify-between">
          <button
            type="button"
            onClick={handleDelete}
            disabled={deleting}
            class="px-4 py-2 text-sm font-medium text-red-600 hover:text-red-700 disabled:opacity-50"
          >
            {deleting ? "Deleting..." : "Delete Spool"}
          </button>
          <button
            type="submit"
            disabled={saving}
            class="px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-primary-600 hover:bg-primary-700 disabled:opacity-50"
          >
            {saving ? "Saving..." : "Save Changes"}
          </button>
        </div>
      </form>
    </div>
  );
}
