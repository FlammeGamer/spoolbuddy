import { useEffect, useState } from "preact/hooks";
import { Link } from "wouter-preact";
import { api, Spool } from "../lib/api";

export function Inventory() {
  const [spools, setSpools] = useState<Spool[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [materialFilter, setMaterialFilter] = useState("");
  const [showAddModal, setShowAddModal] = useState(false);

  useEffect(() => {
    loadSpools();
  }, [search, materialFilter]);

  const loadSpools = async () => {
    try {
      setLoading(true);
      const data = await api.listSpools({
        search: search || undefined,
        material: materialFilter || undefined,
      });
      setSpools(data);
    } catch (e) {
      console.error("Failed to load spools:", e);
    } finally {
      setLoading(false);
    }
  };

  // Get unique materials for filter
  const materials = [...new Set(spools.map((s) => s.material))].sort();

  return (
    <div class="space-y-6">
      {/* Header */}
      <div class="flex justify-between items-center">
        <div>
          <h1 class="text-3xl font-bold text-gray-900">Inventory</h1>
          <p class="text-gray-600">Manage your filament spools</p>
        </div>
        <button
          onClick={() => setShowAddModal(true)}
          class="inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-primary-600 hover:bg-primary-700"
        >
          <svg class="w-5 h-5 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
          </svg>
          Add Spool
        </button>
      </div>

      {/* Filters */}
      <div class="bg-white rounded-lg shadow p-4">
        <div class="flex flex-col md:flex-row gap-4">
          <div class="flex-1">
            <label class="sr-only" htmlFor="search">
              Search
            </label>
            <div class="relative">
              <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                <svg class="h-5 w-5 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                </svg>
              </div>
              <input
                id="search"
                type="text"
                placeholder="Search by color, brand..."
                value={search}
                onInput={(e) => setSearch((e.target as HTMLInputElement).value)}
                class="block w-full pl-10 pr-3 py-2 border border-gray-300 rounded-md leading-5 bg-white placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-primary-500 focus:border-primary-500"
              />
            </div>
          </div>
          <div class="md:w-48">
            <label class="sr-only" htmlFor="material">
              Material
            </label>
            <select
              id="material"
              value={materialFilter}
              onChange={(e) => setMaterialFilter((e.target as HTMLSelectElement).value)}
              class="block w-full px-3 py-2 border border-gray-300 rounded-md leading-5 bg-white focus:outline-none focus:ring-1 focus:ring-primary-500 focus:border-primary-500"
            >
              <option value="">All Materials</option>
              {materials.map((m) => (
                <option key={m} value={m}>
                  {m}
                </option>
              ))}
            </select>
          </div>
        </div>
      </div>

      {/* Spool list */}
      <div class="bg-white rounded-lg shadow overflow-hidden">
        {loading ? (
          <div class="p-8 text-center text-gray-500">Loading...</div>
        ) : spools.length === 0 ? (
          <div class="p-8 text-center text-gray-500">
            {search || materialFilter
              ? "No spools match your filters"
              : "No spools in inventory. Add your first spool!"}
          </div>
        ) : (
          <div class="overflow-x-auto">
            <table class="min-w-full divide-y divide-gray-200">
              <thead class="bg-gray-50">
                <tr>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Color
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Material
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Brand
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Weight
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                    Tag
                  </th>
                  <th class="relative px-6 py-3">
                    <span class="sr-only">Actions</span>
                  </th>
                </tr>
              </thead>
              <tbody class="bg-white divide-y divide-gray-200">
                {spools.map((spool) => (
                  <tr key={spool.id} class="hover:bg-gray-50">
                    <td class="px-6 py-4 whitespace-nowrap">
                      <div class="flex items-center">
                        {spool.rgba && (
                          <div
                            class="w-8 h-8 rounded-full border border-gray-200 mr-3"
                            style={{ backgroundColor: `#${spool.rgba.slice(0, 6)}` }}
                          />
                        )}
                        <span class="text-sm font-medium text-gray-900">
                          {spool.color_name || "Unknown"}
                        </span>
                      </div>
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap">
                      <span class="text-sm text-gray-900">{spool.material}</span>
                      {spool.subtype && (
                        <span class="text-sm text-gray-500"> {spool.subtype}</span>
                      )}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                      {spool.brand || "-"}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                      {spool.weight_current !== null ? (
                        <span>
                          {spool.weight_current}g
                          {spool.label_weight && (
                            <span class="text-gray-400">
                              {" "}
                              / {spool.label_weight}g
                            </span>
                          )}
                        </span>
                      ) : (
                        spool.label_weight ? `${spool.label_weight}g` : "-"
                      )}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap">
                      {spool.tag_id ? (
                        <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-green-100 text-green-800">
                          Tagged
                        </span>
                      ) : (
                        <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-gray-100 text-gray-600">
                          No tag
                        </span>
                      )}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                      <Link
                        href={`/spool/${spool.id}`}
                        class="text-primary-600 hover:text-primary-900"
                      >
                        Edit
                      </Link>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {/* Add spool modal placeholder */}
      {showAddModal && (
        <AddSpoolModal
          onClose={() => setShowAddModal(false)}
          onCreated={() => {
            setShowAddModal(false);
            loadSpools();
          }}
        />
      )}
    </div>
  );
}

interface AddSpoolModalProps {
  onClose: () => void;
  onCreated: () => void;
}

function AddSpoolModal({ onClose, onCreated }: AddSpoolModalProps) {
  const [material, setMaterial] = useState("PLA");
  const [colorName, setColorName] = useState("");
  const [brand, setBrand] = useState("");
  const [labelWeight, setLabelWeight] = useState("1000");
  const [saving, setSaving] = useState(false);

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setSaving(true);

    try {
      await api.createSpool({
        material,
        color_name: colorName || null,
        brand: brand || null,
        label_weight: labelWeight ? parseInt(labelWeight) : null,
      });
      onCreated();
    } catch (e) {
      console.error("Failed to create spool:", e);
      alert("Failed to create spool");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div class="bg-white rounded-lg shadow-xl max-w-md w-full mx-4">
        <div class="px-6 py-4 border-b border-gray-200">
          <h2 class="text-lg font-semibold text-gray-900">Add Spool</h2>
        </div>
        <form onSubmit={handleSubmit}>
          <div class="px-6 py-4 space-y-4">
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
          </div>
          <div class="px-6 py-4 border-t border-gray-200 flex justify-end space-x-3">
            <button
              type="button"
              onClick={onClose}
              class="px-4 py-2 text-sm font-medium text-gray-700 hover:text-gray-500"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={saving}
              class="px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-primary-600 hover:bg-primary-700 disabled:opacity-50"
            >
              {saving ? "Saving..." : "Add Spool"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
